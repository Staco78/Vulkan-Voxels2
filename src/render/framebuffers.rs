use std::ops::Index;

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::utils::drop_then_new;

use super::{depth::DepthBuffer, devices::DEVICE, render_pass::RenderPass, swapchain::Swapchain};

#[derive(Debug)]
pub struct Framebuffers {
    framebuffers: Vec<vk::Framebuffer>,
}

impl Framebuffers {
    pub fn new(
        swapchain: &Swapchain,
        render_pass: &RenderPass,
        depth_buffer: &DepthBuffer,
    ) -> Result<Self> {
        let framebuffers = swapchain
            .image_views
            .iter()
            .map(|i| {
                let attachments = &[*i, depth_buffer.view()];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(**render_pass)
                    .attachments(attachments)
                    .width(swapchain.extent.width)
                    .height(swapchain.extent.height)
                    .layers(1);

                unsafe { DEVICE.create_framebuffer(&create_info, None) }
                    .context("Framebuffer creation failed")
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { framebuffers })
    }

    #[inline(always)]
    pub fn count(&self) -> usize {
        self.framebuffers.len()
    }

    #[inline]
    pub fn recreate(
        &mut self,
        swapchain: &Swapchain,
        render_pass: &RenderPass,
        depth_buffer: &DepthBuffer,
    ) -> Result<()> {
        drop_then_new(self, || Self::new(swapchain, render_pass, depth_buffer))
    }
}

impl Drop for Framebuffers {
    fn drop(&mut self) {
        unsafe {
            for &framebuffer in &self.framebuffers {
                DEVICE.destroy_framebuffer(framebuffer, None);
            }
        }
    }
}

impl Index<usize> for Framebuffers {
    type Output = vk::Framebuffer;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        &self.framebuffers[index]
    }
}
