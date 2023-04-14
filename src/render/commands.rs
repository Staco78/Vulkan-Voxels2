use anyhow::{Context, Result};
use vulkanalia::vk::{
    self, CommandPoolCreateInfo, CommandPoolResetFlags, DeviceV1_0, HasBuilder, QueueFlags,
};

use crate::render::queues::get_queue_family;

use super::devices::DEVICE;

#[derive(Debug)]
pub struct CommandPool {
    pool: vk::CommandPool,
}

impl CommandPool {
    pub fn new(physical_device: vk::PhysicalDevice) -> Result<Self> {
        let graphics_family = get_queue_family(physical_device, QueueFlags::GRAPHICS)?
            .expect("A graphics queue should have been found");
        let info = CommandPoolCreateInfo::builder().queue_family_index(graphics_family);
        let pool = unsafe { DEVICE.create_command_pool(&info, None) }
            .context("Command pool creation failed")?;

        Ok(Self { pool })
    }

    pub fn alloc_buffers(&self, count: usize) -> Result<Vec<CommandBuffer>> {
        let info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count as u32);

        let buffers = unsafe { DEVICE.allocate_command_buffers(&info)? };

        let buffers = buffers
            .iter()
            .map(|b| CommandBuffer { buffer: *b })
            .collect();

        Ok(buffers)
    }

    #[inline]
    pub fn reset(&mut self) -> Result<()> {
        unsafe {
            DEVICE
                .reset_command_pool(self.pool, CommandPoolResetFlags::empty())
                .context("Command pool reset failed")?;
        };
        Ok(())
    }

    #[inline]
    pub fn realloc_buffers(
        &mut self,
        buffers: &mut Vec<CommandBuffer>,
        new_count: usize,
    ) -> Result<()> {
        let old_count = buffers.len();
        if old_count == new_count {
            self.reset()?;
            return Ok(());
        }
        if new_count < old_count {
            for buffer in buffers.drain(new_count..) {
                buffer.free(self.pool);
            }
        } else {
            let new_buffs = self.alloc_buffers(new_count - old_count)?;
            buffers.extend(new_buffs);
        }

        self.reset()?;

        Ok(())
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            DEVICE.destroy_command_pool(self.pool, None);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandBuffer {
    pub buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    #[inline]
    pub fn begin(&mut self) -> Result<()> {
        let info = vk::CommandBufferBeginInfo::builder();
        unsafe { DEVICE.begin_command_buffer(self.buffer, &info)? };
        Ok(())
    }

    #[inline]
    pub fn end(&mut self) -> Result<()> {
        unsafe { DEVICE.end_command_buffer(self.buffer)? };
        Ok(())
    }

    #[inline]
    pub fn free(self, pool: vk::CommandPool) {
        unsafe { DEVICE.free_command_buffers(pool, &[self.buffer]) };
    }
}
