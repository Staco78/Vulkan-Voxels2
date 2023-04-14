use anyhow::{anyhow, Context, Result};
use log::trace;
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder, InstanceV1_0, MemoryRequirements};

use crate::render::{devices::DEVICE, instance::INSTANCE};

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
        requirements: MemoryRequirements,
    ) -> Result<Allocation> {
        trace!(target: "allocator", "Alloc {}B of {:?} memory", requirements.size, properties);
        let memory_type_index =
            get_memory_type_index(self.device_memory_properties, properties, requirements)?;
        let info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);
        let memory =
            unsafe { DEVICE.allocate_memory(&info, None) }.context("Failed to allocated")?;

        let alloc = Allocation {
            memory,
            size: requirements.size as usize,
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
}

impl Allocation {
    #[inline(always)]
    pub fn memory(&self) -> vk::DeviceMemory {
        self.memory
    }
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.size
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
