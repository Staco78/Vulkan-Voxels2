use core::slice;
use std::mem::size_of;

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use super::{commands::CommandBuffer, devices::DEVICE, sync::create_fence, Buffer};

#[derive(Debug)]
pub struct StagingBuffer {
    buff: Buffer,
    data: *mut u8,
    pub fence: vk::Fence,
}

impl StagingBuffer {
    pub fn new(size: usize) -> Result<Self> {
        let mut buff = Buffer::new(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
        let data = buff.map()?.as_mut_ptr();
        let fence = create_fence(false)?;
        Ok(Self { buff, data, fence })
    }

    /// Get a mutable slice to the buffer data.
    ///
    /// # Safety
    /// Same as `mem::transmute<[u8], [T]>`.
    #[inline]
    pub unsafe fn data<T>(&mut self) -> &mut [T] {
        let len = self.buff.size() / size_of::<T>();
        unsafe { slice::from_raw_parts_mut(self.data as *mut _, len) }
    }

    pub fn copy_into(
        &self,
        queue: vk::Queue,
        command_buff: &mut CommandBuffer,
        dst: &mut Buffer,
        size: usize,
    ) -> Result<()> {
        self.buff.flush().context("Buffer flush failed")?;
        command_buff.begin().context("Command buff begin failed")?;
        let region = vk::BufferCopy::builder()
            .size(size as u64)
            .src_offset(0)
            .dst_offset(0);
        unsafe { DEVICE.cmd_copy_buffer(**command_buff, self.buff.buffer, dst.buffer, &[region]) };
        command_buff.end().context("Command buff end failed")?;

        let buffers = &[**command_buff];
        let submit_info = vk::SubmitInfo::builder().command_buffers(buffers);
        unsafe { DEVICE.queue_submit(queue, &[submit_info], self.fence) }
            .context("Queue submitting failed")?;

        Ok(())
    }

    #[inline]
    pub fn wait_copy_end(&self) -> Result<()> {
        unsafe { DEVICE.wait_for_fences(&[self.fence], true, u64::MAX) }
            .context("Fence waiting failed")?;
        unsafe { DEVICE.reset_fences(&[self.fence]) }.context("Reset fence failed")?;
        Ok(())
    }
}
