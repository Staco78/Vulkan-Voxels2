use std::mem::size_of_val;

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, Handle, HasBuilder, PipelineCache, ShaderModuleCreateInfo};

use crate::utils::drop_then_new;

use super::{
    descriptors::DescriptorSetLayout, devices::DEVICE, render_pass::RenderPass,
    swapchain::Swapchain, vertex::VertexDescriptor,
};

#[macro_export]
macro_rules! shader_module {
    ($file: expr) => {
        unsafe {
            $crate::utils::with_convert(
                include_bytes!(concat!(env!("OUT_DIR"), "/", $file)),
                |bytes| $crate::render::pipeline::create_shader_module(bytes),
            )
            .context(concat!("Shader module for ", $file, " failed"))
        }
    };
}

#[derive(Debug)]
pub struct PipelineCreationOptions<'a> {
    pub shaders: Vec<(vk::ShaderModule, vk::ShaderStageFlags)>,
    pub cull_mode: vk::CullModeFlags,
    pub polygon_mode: vk::PolygonMode,
    pub descriptors_layouts: Vec<&'a DescriptorSetLayout>,
    pub push_constant_ranges: Vec<vk::PushConstantRange>,
    pub blend_attachment: vk::PipelineColorBlendAttachmentState,
    pub dynamic_state: vk::PipelineDynamicStateCreateInfo,
}

#[derive(Debug)]
pub struct Pipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
}

impl Pipeline {
    pub fn new<V: VertexDescriptor>(
        swapchain: &Swapchain,
        render_pass: &RenderPass,
        options: &PipelineCreationOptions,
    ) -> Result<Self> {
        let stages: Vec<_> = options
            .shaders
            .iter()
            .map(|&(module, stage)| {
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(stage)
                    .module(module)
                    .name(b"main\0")
            })
            .collect();

        let binding_descriptions = &[V::binding_description()];
        let attribute_descriptions = V::attribute_descriptions();
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
            .polygon_mode(options.polygon_mode)
            .line_width(1.0)
            .cull_mode(options.cull_mode)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);
        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::_1);
        let attachments = &[options.blend_attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);
        let stencil_op = vk::StencilOpState::builder()
            .fail_op(vk::StencilOp::KEEP)
            .pass_op(vk::StencilOp::KEEP)
            .compare_op(vk::CompareOp::ALWAYS);
        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .front(stencil_op)
            .back(stencil_op);

        let layouts = options
            .descriptors_layouts
            .iter()
            .map(|&desc| **desc)
            .collect::<Vec<_>>();
        let layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&layouts)
            .push_constant_ranges(&options.push_constant_ranges);
        let layout = unsafe {
            DEVICE
                .create_pipeline_layout(&layout_info, None)
                .context("Pipeline layout creation failed")?
        };

        let info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(layout)
            .render_pass(**render_pass)
            .subpass(0)
            .depth_stencil_state(&depth_stencil_state)
            .dynamic_state(&options.dynamic_state);

        let pipeline =
            unsafe { DEVICE.create_graphics_pipelines(PipelineCache::null(), &[info], None) }
                .context("Pipeline creation failed")?
                .0;

        unsafe {
            for &(module, _) in &options.shaders {
                DEVICE.destroy_shader_module(module, None);
            }
        };

        Ok(Self { pipeline, layout })
    }

    #[inline]
    pub fn recreate<V: VertexDescriptor>(
        &mut self,
        swapchain: &Swapchain,
        render_pass: &RenderPass,
        options: &PipelineCreationOptions,
    ) -> Result<()> {
        drop_then_new(self, || Self::new::<V>(swapchain, render_pass, options))
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            DEVICE.destroy_pipeline(self.pipeline, None);
            DEVICE.destroy_pipeline_layout(self.layout, None);
        }
    }
}

pub fn create_shader_module(bytes: &[u32]) -> Result<vk::ShaderModule> {
    let info = ShaderModuleCreateInfo::builder()
        .code(bytes)
        .code_size(size_of_val(bytes));
    let module = unsafe { DEVICE.create_shader_module(&info, None) }
        .context("Shader module creation failed")?;
    Ok(module)
}
