use std::ops::Index;

use anyhow::{Context, Result};
use vulkanalia::{
    vk::{self, DeviceV1_0, HasBuilder},
    Device,
};

use super::{pipeline::Pipeline, swapchain::Swapchain};

#[derive(Debug)]
pub struct Framebuffers {
    framebuffers: Vec<vk::Framebuffer>,
}

impl Framebuffers {
    pub fn new(device: &Device, swapchain: &Swapchain, pipeline: &Pipeline) -> Result<Self> {
        let framebuffers = swapchain
            .image_views
            .iter()
            .map(|i| {
                let attachments = &[*i];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(pipeline.render_pass)
                    .attachments(attachments)
                    .width(swapchain.extent.width)
                    .height(swapchain.extent.height)
                    .layers(1);

                unsafe { device.create_framebuffer(&create_info, None) }
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
        device: &Device,
        swapchain: &Swapchain,
        pipeline: &Pipeline,
    ) -> Result<()> {
        self.destroy(device);
        let new = Self::new(device, swapchain, pipeline)?;
        *self = new;
        Ok(())
    }

    pub fn destroy(&mut self, device: &Device) {
        unsafe {
            for &framebuffer in &self.framebuffers {
                device.destroy_framebuffer(framebuffer, None);
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
