use std::{ptr, slice};

use anyhow::{bail, Context, Result};
use log::trace;
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder, InstanceV1_0};

use crate::render::{instance::INSTANCE, memory::get_memory_type_index, DEVICE};

use super::allocator;

#[derive(Debug)]
pub struct Allocator {
    device_memory_properties: vk::PhysicalDeviceMemoryProperties,
}

impl Allocator {
    pub fn new(physical_device: vk::PhysicalDevice) -> Self {
        let device_memory_properties =
            unsafe { INSTANCE.get_physical_device_memory_properties(physical_device) };
        Self {
            device_memory_properties,
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
        let info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);
        let memory =
            unsafe { DEVICE.allocate_memory(&info, None) }.context("Failed to allocated")?;

        let ptr = if mapped {
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

        let alloc = Allocation {
            memory,
            size: requirements.size as usize,
            ptr,
        };
        Ok(alloc)
    }

    #[inline]
    fn free(&self, alloc: &Allocation) {
        trace!(target: "allocator", "Free {}B", alloc.size);
        unsafe { DEVICE.free_memory(alloc.memory, None) }
    }
}

#[derive(Debug)]
pub struct Allocation {
    memory: vk::DeviceMemory,
    size: usize,
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
        0
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
        if self.ptr.is_null() {
            bail!("A non-mapped allocation couldn't be flushed");
        }
        let memory_ranges = &[vk::MappedMemoryRange::builder()
            .memory(self.memory)
            .offset(0)
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
