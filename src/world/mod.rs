mod blocks;
pub mod chunk;
mod pos;

use anyhow::{Context, Result};
pub use pos::*;

use std::{
    collections::HashMap,
    mem::size_of,
    ops::Deref,
    sync::{Mutex, RwLock},
};

use crate::render::{Buffer, CommandBuffer, CommandPool, Queue, StagingBuffer, Vertex, QUEUES};

use vulkanalia::vk;

use self::chunk::Chunk;

pub const CHUNK_SIZE: usize = 32;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const MAX_VERTICES_PER_CHUNK: usize = BLOCKS_PER_CHUNK * 18;
pub const RENDER_DISTANCE: usize = 2;

#[derive(Debug)]
pub struct World {
    chunks: RwLock<HashMap<ChunkPos, Chunk>>,
    staging_buffer: Mutex<StagingBuffer>,

    transfer_queue: Queue,
    _command_pool: CommandPool,
    command_buff: Mutex<CommandBuffer>,
}

impl World {
    pub fn new() -> Result<World> {
        let staging_buffer = StagingBuffer::new(MAX_VERTICES_PER_CHUNK * size_of::<Vertex>())
            .context("Staging buffer creation failed")?;

        let transfer_queue = QUEUES.fetch_queue(vk::QueueFlags::TRANSFER)?;
        let command_pool =
            CommandPool::new(transfer_queue.family).context("Command pool creation failed")?;
        let command_buff = command_pool
            .alloc_buffers(1)
            .context("Command buffer allocation failed")?
            .drain(..)
            .next()
            .expect("There should be 1 command buffer returned");

        Ok(Self {
            chunks: RwLock::new(HashMap::new()),
            staging_buffer: Mutex::new(staging_buffer),

            transfer_queue,
            _command_pool: command_pool,
            command_buff: Mutex::new(command_buff),
        })
    }

    pub fn tick(&self, player_pos: EntityPos) -> Result<()> {
        let player_chunk_pos = player_pos.chunk();
        let (px, py, pz) = player_chunk_pos.xyz();
        let mut chunks = self.chunks.write().expect("Lock poisoned");
        for x in (px - RENDER_DISTANCE as i64)..=(px + RENDER_DISTANCE as i64) {
            for y in (py - RENDER_DISTANCE as i64)..=(py + RENDER_DISTANCE as i64) {
                for z in (pz - RENDER_DISTANCE as i64)..=(pz + RENDER_DISTANCE as i64) {
                    if y > 10 {
                        continue;
                    }
                    let chunk_pos = ChunkPos::new(x, y, z);
                    if chunks.contains_key(&chunk_pos) {
                        continue;
                    }
                    let chunk = self.create_chunk(chunk_pos)?;
                    chunks.insert(chunk_pos, chunk);
                }
            }
        }

        Ok(())
    }

    fn create_chunk(&self, chunk_pos: ChunkPos) -> Result<Chunk> {
        let mut chunk = Chunk::generate(chunk_pos);
        let mut staging_buffer = self.staging_buffer.lock().expect("Mutex poisoned");
        let vertices = unsafe { staging_buffer.data() };
        let vertices_count = chunk.mesh(vertices);
        let mut vertex_buff = Buffer::new(
            vertices_count * size_of::<Vertex>(),
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .context("Vertex buffer creation failed")?;
        let mut command_buff = self.command_buff.lock().expect("Mutex poisoned");
        staging_buffer
            .copy_into(
                *self.transfer_queue,
                &mut command_buff,
                &mut vertex_buff,
                vertices_count * size_of::<Vertex>(),
            )
            .context("Buffer copy failed")?;
        staging_buffer.wait_copy_end()?;
        chunk.vertex_buffer = Some(vertex_buff);
        Ok(chunk)
    }

    #[inline(always)]
    pub fn chunks(&self) -> impl Deref<Target = HashMap<ChunkPos, Chunk>> + '_ {
        self.chunks.read().expect("Lock poisoned")
    }
}
