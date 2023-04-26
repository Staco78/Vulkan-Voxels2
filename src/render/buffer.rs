use anyhow::{anyhow, Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::render::memory::allocator;

use super::{devices::DEVICE, memory::Allocation};

#[derive(Debug)]
pub struct Buffer {
    pub buffer: vk::Buffer,
    pub(in crate::render) alloc: Allocation,
}

impl Buffer {
    pub fn new(
        size: usize,
        usage: vk::BufferUsageFlags,
        alloc_properties: vk::MemoryPropertyFlags,
        mapped: bool,
    ) -> Result<Self> {
        let info = vk::BufferCreateInfo::builder()
            .size(size as u64)
            .usage(usage);
        let buffer =
            unsafe { DEVICE.create_buffer(&info, None) }.context("Buffer creation failed")?;

        let requirements = unsafe { DEVICE.get_buffer_memory_requirements(buffer) };

        let alloc = allocator()
            .alloc(alloc_properties, requirements, mapped)
            .context("Memory allocation failed")?;

        unsafe { DEVICE.bind_buffer_memory(buffer, alloc.memory(), alloc.offset() as u64) }
            .context("Buffer binding failed")?;

        Ok(Self { buffer, alloc })
    }

    #[inline]
    pub fn data(&mut self) -> Result<&mut [u8]> {
        self.alloc
            .data()
            .ok_or_else(|| anyhow!("Buffer has not been created with mapped as true"))
    }

    #[inline(always)]
    pub fn flush(&self) -> Result<()> {
        self.alloc.flush()
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
