use std::ops::{Deref, DerefMut};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use super::devices::DEVICE;

#[derive(Debug)]
pub struct Semaphores {
    semaphores: Vec<vk::Semaphore>,
}

impl Semaphores {
    pub fn new(count: usize) -> Result<Self> {
        let info = vk::SemaphoreCreateInfo::builder();
        let mut semaphores = Vec::with_capacity(count);
        for _ in 0..count {
            unsafe {
                semaphores.push(
                    DEVICE
                        .create_semaphore(&info, None)
                        .context("Semaphore creation failed")?,
                )
            };
        }

        Ok(Self { semaphores })
    }
}

impl Drop for Semaphores {
    fn drop(&mut self) {
        for &semaphore in &self.semaphores {
            unsafe { DEVICE.destroy_semaphore(semaphore, None) };
        }
    }
}

impl Deref for Semaphores {
    type Target = Vec<vk::Semaphore>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.semaphores
    }
}
impl DerefMut for Semaphores {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.semaphores
    }
}

#[inline]
pub fn create_fence(signaled: bool) -> Result<vk::Fence> {
    let flags = if signaled {
        vk::FenceCreateFlags::SIGNALED
    } else {
        vk::FenceCreateFlags::empty()
    };
    let info = vk::FenceCreateInfo::builder().flags(flags);
    unsafe { DEVICE.create_fence(&info, None) }.context("Fence creation failed")
}

#[derive(Debug)]
pub struct Fences {
    fences: Vec<vk::Fence>,
}

impl Fences {
    pub fn new(count: usize, signaled: bool) -> Result<Self> {
        let flags = if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        };
        let info = vk::FenceCreateInfo::builder().flags(flags);
        let mut fences = Vec::with_capacity(count);
        for _ in 0..count {
            unsafe {
                fences.push(
                    DEVICE
                        .create_fence(&info, None)
                        .context("Fence creation failed")?,
                )
            };
        }

        Ok(Self { fences })
    }

    #[inline(always)]
    pub fn from_vec(fences: Vec<vk::Fence>) -> Self {
        Self { fences }
    }
}

impl Drop for Fences {
    fn drop(&mut self) {
        for &fence in &self.fences {
            unsafe { DEVICE.destroy_fence(fence, None) };
        }
    }
}

impl Deref for Fences {
    type Target = Vec<vk::Fence>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.fences
    }
}
impl DerefMut for Fences {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fences
    }
}
