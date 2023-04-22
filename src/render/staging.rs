use core::slice;
use std::mem::{align_of, size_of};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use super::{commands::CommandBuffer, devices::DEVICE, Buffer};

#[derive(Debug)]
pub struct StagingBuffer {
    buff: Buffer,
    data: *mut u8,
}

impl StagingBuffer {
    pub fn new(size: usize) -> Result<Self> {
        let mut buff = Buffer::new(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
        let data = buff.map()?.as_mut_ptr();
        Ok(Self { buff, data })
    }

    /// Get a mutable slice to the buffer data.
    ///
    /// # Safety
    /// Same as `mem::transmute<[u8], [T]>`.
    #[inline]
    pub unsafe fn data<T>(&mut self) -> &mut [T] {
        assert_eq!(self.data as usize % align_of::<T>(), 0);
        let len = self.buff.size() / size_of::<T>();
        unsafe { slice::from_raw_parts_mut(self.data as *mut _, len) }
    }

    pub fn copy_into(
        &self,
        queue: vk::Queue,
        command_buff: &mut CommandBuffer,
        fence: vk::Fence,
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
        unsafe { DEVICE.queue_submit(queue, &[submit_info], fence) }
            .context("Queue submitting failed")?;

        Ok(())
    }
}
