use std::mem::{size_of, size_of_val};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, Handle, HasBuilder, PipelineCache, ShaderModuleCreateInfo};

use crate::{
    options::AppOptions,
    utils::{self, drop_then_new},
    world::ChunkPos,
};

use super::{
    camera::UniformBufferObject, depth::DepthBuffer, devices::DEVICE, swapchain::Swapchain,
    uniform::Uniforms, vertex::Vertex,
};

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
    pub fn new(
        physical_device: vk::PhysicalDevice,
        swapchain: &Swapchain,
        uniforms: &Uniforms<UniformBufferObject>,
    ) -> Result<Self> {
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
            .polygon_mode(AppOptions::get().polygon_mode)
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
        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        let vert_push_constant_range = vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(size_of::<ChunkPos>() as u32);

        let layouts = [uniforms.descriptor_layout];
        let push_constant_ranges = [vert_push_constant_range];
        let layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&layouts)
            .push_constant_ranges(&push_constant_ranges);
        let layout = unsafe {
            DEVICE
                .create_pipeline_layout(&layout_info, None)
                .context("Pipeline layout creation failed")?
        };

        let render_pass = create_render_pass(physical_device, swapchain)
            .context("Render pass creation failed")?;

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
            .subpass(0)
            .depth_stencil_state(&depth_stencil_state);

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
    pub fn recreate(
        &mut self,
        physical_device: vk::PhysicalDevice,
        swapchain: &Swapchain,
        uniforms: &Uniforms<UniformBufferObject>,
    ) -> Result<()> {
        drop_then_new(self, || Self::new(physical_device, swapchain, uniforms))
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

fn create_render_pass(
    physical_device: vk::PhysicalDevice,
    swapchain: &Swapchain,
) -> Result<vk::RenderPass> {
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

    let depth_stencil_attachment = vk::AttachmentDescription::builder()
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
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let depth_stencil_attachment_ref = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let color_attachments = &[color_attachment_ref];
    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(color_attachments)
        .depth_stencil_attachment(&depth_stencil_attachment_ref);

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

    let attachments = &[color_attachment, depth_stencil_attachment];
    let subpasses = &[subpass];
    let dependencies = &[dependency];
    let info = vk::RenderPassCreateInfo::builder()
        .attachments(attachments)
        .subpasses(subpasses)
        .dependencies(dependencies);

    let render_pass = unsafe { DEVICE.create_render_pass(&info, None)? };
    Ok(render_pass)
}
