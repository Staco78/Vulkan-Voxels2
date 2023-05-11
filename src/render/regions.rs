use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    mem::size_of,
    ops::DerefMut,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0};

use crate::render::{CommandBuffer, Vertex, DEVICE};

use crate::world::{chunks::Chunks, ChunkPos, RegionPos, REGION_SIZE};

use super::{pipeline::Pipeline, CommandPool, QUEUES};

#[derive(Debug)]
pub struct RegionCmdBuff {
    pub pos: RegionPos,
    buffers: Vec<CommandBuffer>,
    dirty_buffs: Vec<bool>,
    chunks: Arc<RwLock<Chunks>>,

    min_pos: ChunkPos, // included
    max_pos: ChunkPos, // excluded
}

impl RegionCmdBuff {
    pub fn new(pos: RegionPos, buffers: Vec<CommandBuffer>, chunks: Arc<RwLock<Chunks>>) -> Self {
        let min_pos = ChunkPos::new(
            pos.x() * REGION_SIZE as i64,
            pos.y() * REGION_SIZE as i64,
            pos.z() * REGION_SIZE as i64,
        );
        let max_pos = ChunkPos::new(
            (pos.x() + 1) * REGION_SIZE as i64,
            (pos.y() + 1) * REGION_SIZE as i64,
            (pos.z() + 1) * REGION_SIZE as i64,
        );
        let buffs_count = buffers.len();
        Self {
            pos,
            buffers,
            dirty_buffs: vec![true; buffs_count],
            chunks,

            min_pos,
            max_pos,
        }
    }

    fn record_commands(
        &mut self,
        index: usize,
        pipeline: &Pipeline,
        descriptor_set: vk::DescriptorSet,
        inheritance_info: &vk::CommandBufferInheritanceInfo,
    ) -> Result<()> {
        let buff = &mut self.buffers[index];
        buff.reset()?;
        buff.begin_secondary(inheritance_info)?;

        unsafe {
            DEVICE.cmd_bind_pipeline(**buff, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
            DEVICE.cmd_bind_descriptor_sets(
                **buff,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout,
                0,
                &[descriptor_set],
                &[],
            );
        }
        let chunks = self.chunks.read().expect("Lock poisoned");
        // TODO: using another data structure may permit to get directly an iterator over the required chunks instead of filtering
        for (pos, chunk) in chunks
            .iter()
            .filter(|&(pos, _)| pos.between(&self.min_pos, &self.max_pos))
        {
            debug_assert_eq!(pos.region(), self.pos);
            let Some(ref vertex_buffer) = *chunk.vertex_buffer.lock().expect("Lock poisoned") else { continue; };
            unsafe {
                DEVICE.cmd_bind_vertex_buffers(**buff, 0, &[vertex_buffer.buffer], &[0]);
                DEVICE.cmd_push_constants(
                    **buff,
                    pipeline.layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    pos.as_bytes(),
                );
                let vertices_count = vertex_buffer.size() / size_of::<Vertex>();
                DEVICE.cmd_draw(**buff, vertices_count as u32, 1, 0, 0);
            }
        }

        buff.end()?;
        Ok(())
    }

    pub fn fetch_cmd_buff(
        &mut self,
        index: usize,
        pipeline: &Pipeline,
        descriptor_set: vk::DescriptorSet,
        inheritance_info: &vk::CommandBufferInheritanceInfo,
    ) -> Result<vk::CommandBuffer> {
        if self.dirty_buffs[index] {
            self.dirty_buffs[index] = false;
            self.record_commands(index, pipeline, descriptor_set, inheritance_info)?;
        }
        Ok(*self.buffers[index])
    }

    #[inline]
    pub fn set_dirty(&mut self) {
        self.dirty_buffs.fill(true);
    }
}

#[derive(Debug)]
pub struct RegionsManager {
    regions: Mutex<HashMap<RegionPos, RegionCmdBuff>>,
    chunks: Arc<RwLock<Chunks>>,
    pool: Mutex<CommandPool>,
    buffers_count: AtomicUsize,
}

impl RegionsManager {
    pub fn new(chunks: Arc<RwLock<Chunks>>, buffers_count: usize) -> Result<Self> {
        assert!(buffers_count <= usize::BITS as _);
        let pool = Mutex::new(CommandPool::new(QUEUES.get_default_graphics().family)?);
        Ok(Self {
            regions: Mutex::new(HashMap::new()),
            chunks,
            pool,
            buffers_count: AtomicUsize::new(buffers_count),
        })
    }

    fn create_region(&self, pos: RegionPos) -> Result<RegionCmdBuff> {
        let mut pool = self.pool.lock().expect("Mutex poisoned");
        let buffers = pool
            .alloc_buffers(self.buffers_count.load(Ordering::Relaxed), true)
            .context("Command buffers allocation failed")?;
        Ok(RegionCmdBuff::new(pos, buffers, Arc::clone(&self.chunks)))
    }

    pub fn set_dirty(&self, pos: RegionPos) -> Result<()> {
        let mut regions = self.regions.lock().expect("Mutex poisoned");
        let mut entry = regions.entry(pos);
        let region = match entry {
            Entry::Occupied(ref mut entry) => entry.get_mut(),
            Entry::Vacant(entry) => {
                let val = self.create_region(pos).context("Region creation failed")?;
                entry.insert(val)
            }
        };
        region.set_dirty();
        Ok(())
    }

    pub fn inner(&self) -> impl DerefMut<Target = HashMap<RegionPos, RegionCmdBuff>> + '_ {
        self.regions.lock().expect("Mutex poisoned")
    }

    pub fn pipeline_recreated(&self, new_count: usize) -> Result<()> {
        let mut pool = self.pool.lock().expect("Mutex poisoned");
        self.buffers_count.store(new_count, Ordering::Relaxed);
        let mut regions = self.inner();
        for region in regions.values_mut() {
            pool.realloc_buffers(&mut region.buffers, new_count, true)?;
            region.dirty_buffs.resize(new_count, true);
            region.set_dirty();
        }
        Ok(())
    }
}
