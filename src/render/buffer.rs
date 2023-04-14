use std::{mem::size_of_val, ptr};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder, MemoryPropertyFlags};

use crate::render::memory::allocator;

use super::{devices::DEVICE, memory::Allocation};

#[derive(Debug)]
pub struct Buffer {
    pub buffer: vk::Buffer,
    alloc: Allocation,
}

impl Buffer {
    pub fn create(size: usize, usage: vk::BufferUsageFlags) -> Result<Self> {
        let info = vk::BufferCreateInfo::builder()
            .size(size as u64)
            .usage(usage);
        let buffer =
            unsafe { DEVICE.create_buffer(&info, None) }.context("Buffer creation failed")?;

        let requirements = unsafe { DEVICE.get_buffer_memory_requirements(buffer) };

        let alloc = allocator()
            .alloc(MemoryPropertyFlags::HOST_VISIBLE, requirements)
            .context("Memory allocation failed")?;

        unsafe { DEVICE.bind_buffer_memory(buffer, alloc.memory(), 0) }
            .context("Buffer binding failed")?;

        Ok(Self { buffer, alloc })
    }

    pub fn fill<T>(&mut self, data: &[T]) -> Result<()> {
        let memory = unsafe {
            DEVICE.map_memory(
                self.alloc.memory(),
                0,
                vk::WHOLE_SIZE as u64,
                vk::MemoryMapFlags::empty(),
            )
        }
        .context("Memory mapping failed")?;

        let size = size_of_val(data);
        assert!(size <= self.alloc.size());
        unsafe { ptr::copy_nonoverlapping(data.as_ptr(), memory.cast(), size) };

        let memory_ranges = &[vk::MappedMemoryRange::builder()
            .memory(self.alloc.memory())
            .offset(0)
            .size(vk::WHOLE_SIZE as u64)];

        unsafe {
            DEVICE
                .flush_mapped_memory_ranges(memory_ranges)
                .context("Memory ranges flush failed")?;
            DEVICE.unmap_memory(self.alloc.memory());
        };

        Ok(())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { DEVICE.destroy_buffer(self.buffer, None) };
    }
}
