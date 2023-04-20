use core::slice;

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::render::memory::allocator;

use super::{devices::DEVICE, memory::Allocation};

#[derive(Debug)]
pub struct Buffer {
    pub buffer: vk::Buffer,
    alloc: Allocation,
}

impl Buffer {
    pub fn new(
        size: usize,
        usage: vk::BufferUsageFlags,
        alloc_properties: vk::MemoryPropertyFlags,
    ) -> Result<Self> {
        let info = vk::BufferCreateInfo::builder()
            .size(size as u64)
            .usage(usage);
        let buffer =
            unsafe { DEVICE.create_buffer(&info, None) }.context("Buffer creation failed")?;

        let requirements = unsafe { DEVICE.get_buffer_memory_requirements(buffer) };

        let alloc = allocator()
            .alloc(alloc_properties, requirements)
            .context("Memory allocation failed")?;

        unsafe { DEVICE.bind_buffer_memory(buffer, alloc.memory(), 0) }
            .context("Buffer binding failed")?;

        Ok(Self { buffer, alloc })
    }

    pub fn map(&mut self) -> Result<&mut [u8]> {
        let memory = unsafe {
            DEVICE.map_memory(
                self.alloc.memory(),
                0,
                vk::WHOLE_SIZE as u64,
                vk::MemoryMapFlags::empty(),
            )
        }
        .context("Memory mapping failed")?;

        let len = self.alloc.size();
        let slice = unsafe { slice::from_raw_parts_mut(memory.cast(), len) };
        Ok(slice)
    }

    pub fn flush(&self) -> Result<()> {
        let memory_ranges = &[vk::MappedMemoryRange::builder()
            .memory(self.alloc.memory())
            .offset(0)
            .size(vk::WHOLE_SIZE as u64)];

        unsafe {
            DEVICE
                .flush_mapped_memory_ranges(memory_ranges)
                .context("Memory ranges flush failed")?;
        };

        Ok(())
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.alloc.size()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { DEVICE.destroy_buffer(self.buffer, None) };
    }
}
