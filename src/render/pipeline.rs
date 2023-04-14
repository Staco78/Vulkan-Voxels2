use std::{mem::size_of_val, ptr};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, Handle, HasBuilder, PipelineCache, ShaderModuleCreateInfo};

use crate::utils;

use super::{devices::DEVICE, swapchain::Swapchain, vertex::Vertex};

macro_rules! shader_module {
    ($file: expr) => {
        unsafe {
            utils::with_convert(
                include_bytes!(concat!(env!("OUT_DIR"), "/", $file)),
                |bytes| create_shader_module(bytes),
            )
            .context(concat!("Shader module for ", $file, " failed"))
        }
    };
}

#[derive(Debug)]
pub struct Pipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub render_pass: vk::RenderPass,
}

impl Pipeline {
    pub fn new(swapchain: &Swapchain) -> Result<Self> {
        let frag_module = shader_module!("shader.frag")?;
        let vert_module = shader_module!("shader.vert")?;

        let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(b"main\0");
        let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(b"main\0");

        let binding_descriptions = &[Vertex::binding_description()];
        let attribute_descriptions = Vertex::attribute_descriptions();
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);
        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(swapchain.extent.width as f32)
            .height(swapchain.extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(swapchain.extent);
        let viewports = &[viewport];
        let scissors = &[scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(viewports)
            .scissors(scissors);
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);
        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::_1);
        let attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false);
        let attachments = &[attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let layout_info = vk::PipelineLayoutCreateInfo::builder();
        let layout = unsafe {
            DEVICE
                .create_pipeline_layout(&layout_info, None)
                .context("Pipeline layout creation failed")?
        };

        let render_pass = create_render_pass(swapchain).context("Render pass creation failed")?;

        let stages = &[vert_stage, frag_stage];
        let info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(layout)
            .render_pass(render_pass)
            .subpass(0);

        let pipeline =
            unsafe { DEVICE.create_graphics_pipelines(PipelineCache::null(), &[info], None) }
                .context("Pipeline creation failed")?
                .0;

        unsafe {
            DEVICE.destroy_shader_module(frag_module, None);
            DEVICE.destroy_shader_module(vert_module, None);
        };

        Ok(Self {
            pipeline,
            layout,
            render_pass,
        })
    }

    #[inline]
    pub fn recreate(&mut self, swapchain: &Swapchain) -> Result<()> {
        unsafe { ptr::drop_in_place(self) };
        let new = Self::new(swapchain)?;
        unsafe { ptr::write(self, new) };
        Ok(())
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            DEVICE.destroy_pipeline(self.pipeline, None);
            DEVICE.destroy_pipeline_layout(self.layout, None);
            DEVICE.destroy_render_pass(self.render_pass, None);
        }
    }
}

fn create_shader_module(bytes: &[u32]) -> Result<vk::ShaderModule> {
    let info = ShaderModuleCreateInfo::builder()
        .code(bytes)
        .code_size(size_of_val(bytes));
    let module = unsafe { DEVICE.create_shader_module(&info, None) }
        .context("Shader module creation failed")?;
    Ok(module)
}

fn create_render_pass(swapchain: &Swapchain) -> Result<vk::RenderPass> {
    let color_attachment = vk::AttachmentDescription::builder()
        .format(swapchain.format.format)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let color_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let color_attachments = &[color_attachment_ref];
    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(color_attachments);

    let dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

    let attachments = &[color_attachment];
    let subpasses = &[subpass];
    let dependencies = &[dependency];
    let info = vk::RenderPassCreateInfo::builder()
        .attachments(attachments)
        .subpasses(subpasses)
        .dependencies(dependencies);

    let render_pass = unsafe { DEVICE.create_render_pass(&info, None)? };
    Ok(render_pass)
}
