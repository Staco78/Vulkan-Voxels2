use core::slice;
use std::{
    mem::{align_of, size_of},
    ops::{Deref, DerefMut},
};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use super::{commands::CommandBuffer, devices::DEVICE, Buffer};

#[derive(Debug)]
pub struct StagingBuffer {
    buff: Buffer,
}

impl StagingBuffer {
    pub fn new(size: usize, alignment: usize) -> Result<Self> {
        let buff = Buffer::new(
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE,
            true,
            alignment,
        )?;
        Ok(Self { buff })
    }

    /// Get a mutable slice to the buffer data.
    ///
    /// # Safety
    /// Same as `mem::transmute<[u8], [T]>`.
    #[inline]
    pub unsafe fn data<T>(&mut self) -> &mut [T] {
        let ptr = self
            .buff
            .data()
            .expect("buff has been created with mapped as true")
            .as_mut_ptr();
        assert_eq!(ptr as usize % align_of::<T>(), 0);
        let len = self.buff.size() / size_of::<T>();
        unsafe { slice::from_raw_parts_mut(ptr as *mut _, len) }
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
        command_buff.begin()?;
        let region = vk::BufferCopy::builder()
            .size(size as u64)
            .src_offset(0)
            .dst_offset(0);
        unsafe { DEVICE.cmd_copy_buffer(**command_buff, self.buff.buffer, dst.buffer, &[region]) };
        command_buff.end()?;

        let buffers = &[**command_buff];
        let submit_info = vk::SubmitInfo::builder().command_buffers(buffers);
        unsafe { DEVICE.queue_submit(queue, &[submit_info], fence) }
            .context("Queue submitting failed")?;

        Ok(())
    }
}

impl Deref for StagingBuffer {
    type Target = Buffer;
    fn deref(&self) -> &Self::Target {
        &self.buff
    }
}
impl DerefMut for StagingBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buff
    }
}
