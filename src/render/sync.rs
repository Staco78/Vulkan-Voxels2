use std::ops::{Deref, DerefMut};

use anyhow::{Context, Result};
use vulkanalia::{
    vk::{self, DeviceV1_0, HasBuilder},
    Device,
};

#[derive(Debug)]
pub struct Semaphores {
    semaphores: Vec<vk::Semaphore>,
}

impl Semaphores {
    pub fn new(device: &Device, count: usize) -> Result<Self> {
        let info = vk::SemaphoreCreateInfo::builder();
        let mut semaphores = Vec::with_capacity(count);
        for _ in 0..count {
            unsafe {
                semaphores.push(
                    device
                        .create_semaphore(&info, None)
                        .context("Semaphore creation failed")?,
                )
            };
        }

        Ok(Self { semaphores })
    }

    pub fn destroy(&mut self, device: &Device) {
        for &semaphore in &self.semaphores {
            unsafe { device.destroy_semaphore(semaphore, None) };
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

#[derive(Debug)]
pub struct Fences {
    fences: Vec<vk::Fence>,
}

impl Fences {
    pub fn new(device: &Device, count: usize, signaled: bool) -> Result<Self> {
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
                    device
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

    #[inline]
    pub fn destroy(&mut self, device: &Device) {
        for &fence in &self.fences {
            unsafe { device.destroy_fence(fence, None) };
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
