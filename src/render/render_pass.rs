use std::ops::Deref;

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::utils::drop_then_new;

use super::{depth::DepthBuffer, swapchain::Swapchain, DEVICE};

#[derive(Debug)]
pub struct RenderPassCreationOptions {
    color: vk::AttachmentDescription,
    depth: Option<vk::AttachmentDescription>,
}

impl RenderPassCreationOptions {
    pub fn default(swapchain: &Swapchain) -> Self {
        Self {
            color: vk::AttachmentDescription::builder()
                .format(swapchain.format.format)
                .samples(vk::SampleCountFlags::_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .build(),
            depth: None,
        }
    }

    pub fn with_depth(mut self, physical_device: vk::PhysicalDevice) -> Result<Self> {
        let depth = vk::AttachmentDescription::builder()
            .format(
                DepthBuffer::get_format(physical_device)
                    .context("No valid depth buffer format found")?,
            )
            .samples(vk::SampleCountFlags::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();
        self.depth = Some(depth);
        Ok(self)
    }
}

#[derive(Debug)]
pub struct RenderPass {
    inner: vk::RenderPass,
}

impl RenderPass {
    pub fn new(options: &RenderPassCreationOptions) -> Result<Self> {
        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let depth_stencil_attachment_ref = vk::AttachmentReference::builder()
            .attachment(1)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let color_attachments = &[color_attachment_ref];
        let mut subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(color_attachments);
        if options.depth.is_some() {
            subpass = subpass.depth_stencil_attachment(&depth_stencil_attachment_ref);
        }

        let dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                    | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            )
            .dst_access_mask(
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            );

        let attachments = if let Some(depth) = options.depth {
            vec![options.color, depth]
        } else {
            vec![options.color]
        };
        let subpasses = &[subpass];
        let dependencies = &[dependency];
        let info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(subpasses)
            .dependencies(dependencies);

        let render_pass = unsafe { DEVICE.create_render_pass(&info, None)? };

        Ok(Self { inner: render_pass })
    }

    #[inline]
    pub fn recreate(&mut self, options: &RenderPassCreationOptions) -> Result<()> {
        drop_then_new(self, || Self::new(options))
    }
}

impl Deref for RenderPass {
    type Target = vk::RenderPass;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe { DEVICE.destroy_render_pass(self.inner, None) };
    }
}
