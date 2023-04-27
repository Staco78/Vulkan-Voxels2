use core::slice;
use std::{
    ptr,
    sync::{
        atomic::{AtomicU32, AtomicUsize, Ordering},
        Mutex, RwLock,
    },
};

use anyhow::{anyhow, bail, Context, Result};
use log::trace;
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder, InstanceV1_0};

use crate::render::{devices::DEVICE, instance::INSTANCE};

use super::allocator;

const MIN_CHUNK_SIZE: usize = 1024 * 1024 * 32;

#[derive(Debug)]
pub struct Allocator {
    device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pools: Vec<Pool>,
}

impl Allocator {
    pub fn new(physical_device: vk::PhysicalDevice) -> Self {
        let device_memory_properties =
            unsafe { INSTANCE.get_physical_device_memory_properties(physical_device) };
        let pools = {
            let mut vec = Vec::with_capacity(device_memory_properties.memory_type_count as usize);
            for (i, _) in device_memory_properties
                .memory_types
                .iter()
                .take(device_memory_properties.memory_type_count as usize)
                .enumerate()
            {
                vec.push(Pool::new(i as u32));
            }
            vec
        };
        Self {
            device_memory_properties,
            pools,
        }
    }

    pub fn alloc(
        &self,
        properties: vk::MemoryPropertyFlags,
        requirements: vk::MemoryRequirements,
        mapped: bool,
    ) -> Result<Allocation> {
        trace!(target: "allocator", "Alloc {}B of {:?} memory", requirements.size, properties);
        let memory_type_index =
            get_memory_type_index(self.device_memory_properties, properties, requirements)?;
        let pool = &self.pools[memory_type_index as usize];
        pool.alloc(
            requirements.size as usize,
            requirements.alignment as usize,
            mapped,
        )
        .context("Alloc failed")
    }

    #[inline]
    fn free(&self, alloc: &Allocation) {
        trace!(target: "allocator", "Free {}B", alloc.size);

        let pool = &self.pools[alloc.memory_type_index as usize];
        pool.free(alloc);
    }
}

#[derive(Debug)]
struct Pool {
    memory_type_index: u32,
    chunks_id_counter: AtomicU32,
    chunks: RwLock<Vec<Chunk>>, // sorted by id
}

impl Pool {
    fn new(memory_type_index: u32) -> Self {
        Self {
            memory_type_index,
            chunks_id_counter: AtomicU32::new(0),
            chunks: RwLock::new(Vec::new()),
        }
    }

    fn alloc(&self, size: usize, alignment: usize, mapped: bool) -> Result<Allocation> {
        let chunks = self.chunks.read().expect("Lock poisoned");
        for chunk in chunks.iter() {
            let free_size = chunk.size - chunk.used.load(Ordering::Relaxed);
            if chunk.mapped_ptr.is_null() != mapped && free_size >= size {
                if let Some(alloc) = chunk.try_alloc(size, alignment) {
                    return Ok(alloc);
                }
            }
        }
        drop(chunks);
        let mut new_chunk = {
            let chunk_size = MIN_CHUNK_SIZE.max(size);
            let (allocated_size, memory) = {
                let info = vk::MemoryAllocateInfo::builder()
                    .allocation_size(chunk_size as u64)
                    .memory_type_index(self.memory_type_index);
                let r = unsafe { DEVICE.allocate_memory(&info, None) };
                match r {
                    Ok(memory) => (chunk_size, memory),
                    Err(
                        vk::ErrorCode::OUT_OF_DEVICE_MEMORY | vk::ErrorCode::OUT_OF_HOST_MEMORY,
                    ) => {
                        let info = vk::MemoryAllocateInfo::builder()
                            .allocation_size(size as u64)
                            .memory_type_index(self.memory_type_index);
                        let mem = unsafe { DEVICE.allocate_memory(&info, None) }?;
                        (size, mem)
                    }
                    Err(e) => bail!(e),
                }
            };
            Chunk::new(0, allocated_size, self.memory_type_index, memory, mapped)?
        };
        let mut chunks = self.chunks.write().expect("Lock poisoned");
        // do that here to prevent a chunk with a greater id to be pushed before
        new_chunk.id = self.chunks_id_counter.fetch_add(1, Ordering::Relaxed);
        let alloc = new_chunk
            .try_alloc(size, alignment)
            .expect("Alloc from new chunk should success");
        chunks.push(new_chunk);

        Ok(alloc)
    }

    fn free(&self, alloc: &Allocation) {
        let chunks = self.chunks.read().expect("Lock poisoned");
        let index = chunks
            .binary_search_by(|chunk| chunk.id.cmp(&alloc.chunk_id))
            .expect("Invalid chunk id in allocation when freeing");
        chunks[index].free(alloc);
    }
}

#[derive(Debug)]
struct Chunk {
    id: u32,
    size: usize,
    used: AtomicUsize,
    memory_type_index: u32,
    memory: vk::DeviceMemory,
    blocks: Mutex<Vec<Block>>, // sorted by offset
    mapped_ptr: *mut u8,
}

unsafe impl Send for Chunk {}
unsafe impl Sync for Chunk {}

impl Chunk {
    fn new(
        id: u32,
        size: usize,
        memory_type_index: u32,
        memory: vk::DeviceMemory,
        mapped: bool,
    ) -> Result<Self> {
        let block = Block {
            size,
            offset: 0,
            is_free: true,
        };
        let mapped_ptr = if mapped {
            unsafe {
                DEVICE.map_memory(
                    memory,
                    0,
                    vk::WHOLE_SIZE as u64,
                    vk::MemoryMapFlags::empty(),
                )
            }
            .context("Memory mapping failed")? as *mut u8
        } else {
            ptr::null_mut()
        };
        Ok(Self {
            id,
            size,
            used: AtomicUsize::new(0),
            memory_type_index,
            memory,
            blocks: Mutex::new(vec![block]),
            mapped_ptr,
        })
    }

    fn try_alloc(&self, size: usize, alignment: usize) -> Option<Allocation> {
        let mut blocks = self.blocks.lock().expect("Mutex poisoned");
        for (i, block) in blocks.iter_mut().enumerate() {
            let aligned_size = block.aligned_size(alignment);
            if block.is_free && aligned_size >= size {
                let prev_block = if aligned_size != block.size {
                    Some(Block {
                        offset: block.offset,
                        size: block.size - aligned_size,
                        is_free: true,
                    })
                } else {
                    None
                };
                let new_block = Block {
                    offset: block.offset + (block.size - aligned_size),
                    size,
                    is_free: false,
                };
                let next_block_size =
                    block.size - (prev_block.map(|b| b.size).unwrap_or(0) + new_block.size);
                let next_block = if next_block_size > 0 {
                    Some(Block {
                        offset: new_block.offset + new_block.size,
                        size: next_block_size,
                        is_free: true,
                    })
                } else {
                    None
                };

                let (a, b) = if let Some(prev_block) = prev_block {
                    *block = prev_block;
                    (Some(new_block), next_block)
                } else {
                    *block = new_block;
                    (next_block, None)
                };

                if let Some(a) = a && let Some(b) = b {
                    blocks.splice((i+1)..(i+1), [a, b]);
                }
                else if let Some(a) = a {
                    blocks.insert(i + 1, a);
                } else if let Some(b) = b {
                    blocks.insert(i + 1, b);
                }

                let ptr = if self.mapped_ptr.is_null() {
                    self.mapped_ptr
                } else {
                    unsafe { self.mapped_ptr.add(new_block.offset) }
                };
                let alloc = Allocation {
                    memory_type_index: self.memory_type_index,
                    memory: self.memory,
                    chunk_id: self.id,
                    size,
                    offset: new_block.offset,
                    ptr,
                };
                debug_assert!(alloc.offset < self.size);
                debug_assert!(alloc.offset + alloc.size < self.size);
                self.used.fetch_add(alloc.size, Ordering::Relaxed);
                return Some(alloc);
            }
        }
        None
    }

    fn free(&self, alloc: &Allocation) {
        let mut blocks = self.blocks.lock().expect("Mutex poisoned");
        let index = blocks
            .binary_search_by(|block| block.offset.cmp(&alloc.offset))
            .expect("Invalid allocation offset when freeing");
        debug_assert_eq!(blocks[index].size, alloc.size);
        debug_assert!(!blocks[index].is_free);
        blocks[index].is_free = true;
        self.used.fetch_sub(alloc.size, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy)]
struct Block {
    offset: usize,
    size: usize,
    is_free: bool,
}

impl Block {
    #[inline(always)]
    fn aligned_size(&self, alignment: usize) -> usize {
        self.size.saturating_sub(self.offset % alignment)
    }
}

#[derive(Debug)]
pub struct Allocation {
    memory_type_index: u32,
    memory: vk::DeviceMemory,
    chunk_id: u32,
    size: usize,
    offset: usize,
    ptr: *mut u8,
}

unsafe impl Send for Allocation {}
unsafe impl Sync for Allocation {}

impl Allocation {
    #[inline(always)]
    pub fn memory(&self) -> vk::DeviceMemory {
        self.memory
    }
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.size
    }
    #[inline(always)]
    pub fn offset(&self) -> usize {
        self.offset
    }

    #[inline(always)]
    pub fn data(&mut self) -> Option<&mut [u8]> {
        if !self.ptr.is_null() {
            Some(unsafe { slice::from_raw_parts_mut(self.ptr, self.size) })
        } else {
            None
        }
    }

    #[inline]
    pub fn flush(&self) -> Result<()> {
        let memory_ranges = &[vk::MappedMemoryRange::builder()
            .memory(self.memory)
            .offset(self.offset as u64)
            .size(self.size as u64)];
        unsafe {
            DEVICE
                .flush_mapped_memory_ranges(memory_ranges)
                .context("Allocation flush failed")?;
        };
        Ok(())
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        allocator().free(self)
    }
}

fn get_memory_type_index(
    memory: vk::PhysicalDeviceMemoryProperties,
    properties: vk::MemoryPropertyFlags,
    requirements: vk::MemoryRequirements,
) -> Result<u32> {
    (0..memory.memory_type_count)
        .find(|i| {
            let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
            let memory_type = memory.memory_types[*i as usize];
            suitable && memory_type.property_flags.contains(properties)
        })
        .ok_or_else(|| anyhow!("Failed to find suitable memory type."))
}
