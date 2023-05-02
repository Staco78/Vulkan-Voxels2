use std::ops::Deref;

use anyhow::{Context, Result};
use vulkanalia::vk::{self, CommandPoolCreateInfo, CommandPoolResetFlags, DeviceV1_0, HasBuilder};

use super::{create_fence, devices::DEVICE, Queue};

#[derive(Debug)]
pub struct CommandPool {
    pool: vk::CommandPool,
}

impl CommandPool {
    pub fn new(queue_family: u32) -> Result<Self> {
        let info = CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family)
            .flags(
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                    | vk::CommandPoolCreateFlags::TRANSIENT,
            );
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

#[derive(Debug)]
pub struct CommandBuffer {
    buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    #[inline]
    pub fn begin(&mut self) -> Result<()> {
        let info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            DEVICE
                .begin_command_buffer(self.buffer, &info)
                .context("Command buffer begining failed")?
        };
        Ok(())
    }

    #[inline]
    pub fn end(&mut self) -> Result<()> {
        unsafe {
            DEVICE
                .end_command_buffer(self.buffer)
                .context("Command buffer ending failed")?
        };
        Ok(())
    }

    #[inline]
    fn free(self, pool: vk::CommandPool) {
        unsafe { DEVICE.free_command_buffers(pool, &[self.buffer]) };
    }

    #[inline]
    pub fn reset(&mut self) -> Result<()> {
        unsafe { DEVICE.reset_command_buffer(self.buffer, vk::CommandBufferResetFlags::empty()) }
            .context("Command buffer reset failed")
    }

    pub fn run_one_time_commands<C>(&mut self, queue: &Queue, closure: C) -> Result<()>
    where
        C: FnOnce(vk::CommandBuffer),
    {
        self.begin()?;
        closure(self.buffer);
        self.end()?;

        let buffers = &[self.buffer];
        let submit_info = vk::SubmitInfo::builder().command_buffers(buffers);
        let fence = create_fence(false).context("Fence creation failed")?;
        unsafe {
            DEVICE
                .queue_submit(**queue, &[submit_info], fence)
                .context("Queue submit failed")?;
            DEVICE
                .wait_for_fences(&[fence], false, u64::MAX)
                .context("Failed waiting for fence")?;
        };

        Ok(())
    }
}

impl Deref for CommandBuffer {
    type Target = vk::CommandBuffer;
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}
