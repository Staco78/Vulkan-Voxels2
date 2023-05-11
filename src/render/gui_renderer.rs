use std::{
    collections::HashMap,
    marker::Unsize,
    mem::{align_of, size_of},
};

use anyhow::{Context, Result};
use memoffset::offset_of;
use nalgebra_glm::Vec2;
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::{
    gui,
    render::{
        texture::{Texture, TextureCreationOptions},
        StagingBuffer, DEVICE,
    },
    shader_module,
};

use super::{
    descriptors::{DescriptorPool, DescriptorSetLayout},
    pipeline::{Pipeline, PipelineCreationOptions},
    render_pass::RenderPass,
    swapchain::Swapchain,
    uniform::Uniforms,
    vertex::VertexDescriptor,
    Buffer, CommandBuffer, CommandPool, QUEUES,
};

const DEFAULT_INDEX_BUFFER_SIZE: usize = 2048;
const DEFAULT_VERTEX_BUFFER_SIZE: usize = 4000;
const MAX_TEXTURES: usize = 4;

impl VertexDescriptor for gui::Vertex {
    fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn attribute_descriptions() -> impl Unsize<[vk::VertexInputAttributeDescription]> {
        [
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(gui::Vertex, pos) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(gui::Vertex, uv) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R8G8B8A8_UNORM)
                .offset(offset_of!(gui::Vertex, color) as u32)
                .build(),
        ]
    }
}

#[derive(Debug)]
pub struct GuiRenderer {
    pipeline: Pipeline,
    vertex_buffers: Vec<Buffer>,
    index_buffers: Vec<Buffer>,

    uniforms: Uniforms<Vec2>,
    textures_command_buff: CommandBuffer,

    descriptor_pool: DescriptorPool,
    descriptor_layout: DescriptorSetLayout,
    textures: HashMap<egui::TextureId, Texture>,

    command_pool: CommandPool,
    command_buffers: Vec<CommandBuffer>,
}

impl GuiRenderer {
    pub fn new(
        swapchain: &Swapchain,
        render_pass: &RenderPass,
        textures_cmd_pool: &mut CommandPool,
    ) -> Result<Self> {
        let uniforms = Uniforms::new(swapchain.images.len()).context("Uniforms creation failed")?;

        let pool = DescriptorPool::new(MAX_TEXTURES, vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .context("Descriptor pool creation failed")?;
        let layout = DescriptorSetLayout::new(&Texture::binding(0))
            .context("Descriptor set layout creation failed")?;

        let pipeline_options = Self::pipeline_options(&[&uniforms.layout, &layout])?;
        let pipeline = Pipeline::new::<gui::Vertex>(swapchain, render_pass, &pipeline_options)
            .context("Pipeline creation failed")?;

        let vertex_buffers: Vec<_> = (0..swapchain.image_views.len())
            .map(|_| Self::create_vertex_buff())
            .collect::<Result<Vec<_>>>()
            .context("Vertex buffers creation failed")?;

        let index_buffers: Vec<_> = (0..swapchain.image_views.len())
            .map(|_| Self::create_index_buff())
            .collect::<Result<Vec<_>>>()
            .context("Vertex buffers creation failed")?;

        let textures_command_buff = textures_cmd_pool
            .alloc_buffers(1, false)
            .context("Failed to alloc command buffer")?
            .into_iter()
            .next()
            .expect("Should contain one buffer");

        let mut command_pool = CommandPool::new(QUEUES.get_default_graphics().family)
            .context("Command pool creation failed")?;
        let command_buffers = command_pool
            .alloc_buffers(swapchain.images.len(), true)
            .context("Command buffers allocation failed")?;

        let mut s = Self {
            pipeline,
            vertex_buffers,
            index_buffers,
            uniforms,
            textures_command_buff,

            descriptor_pool: pool,
            descriptor_layout: layout,
            textures: HashMap::new(),

            command_pool,
            command_buffers,
        };
        s.fill_uniforms(swapchain);
        Ok(s)
    }

    fn pipeline_options<'a>(
        layouts: &[&'a DescriptorSetLayout],
    ) -> Result<PipelineCreationOptions<'a>> {
        let mut vec = Vec::with_capacity(layouts.len());
        vec.extend_from_slice(layouts);
        Ok(PipelineCreationOptions {
            shaders: vec![
                (shader_module!("gui.vert")?, vk::ShaderStageFlags::VERTEX),
                (shader_module!("gui.frag")?, vk::ShaderStageFlags::FRAGMENT),
            ],
            cull_mode: vk::CullModeFlags::NONE,
            polygon_mode: vk::PolygonMode::FILL,
            descriptors_layouts: vec,
            push_constant_ranges: Vec::new(),
            blend_attachment: vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::ONE)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_write_mask(vk::ColorComponentFlags::all())
                .build(),
            dynamic_state: vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&[vk::DynamicState::SCISSOR])
                .build(),
        })
    }

    #[inline]
    fn create_vertex_buff() -> Result<Buffer> {
        Buffer::new(
            DEFAULT_VERTEX_BUFFER_SIZE,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            true,
            align_of::<gui::Vertex>(),
        )
    }

    #[inline]
    fn create_index_buff() -> Result<Buffer> {
        Buffer::new(
            DEFAULT_INDEX_BUFFER_SIZE,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            true,
            align_of::<u32>(),
        )
    }

    fn resize_buff<T>(
        buffer: &mut Buffer,
        min_len: usize,
        usage: vk::BufferUsageFlags,
    ) -> Result<()> {
        let min_size = min_len * size_of::<T>();
        let mut new_size = buffer.size() * 2;
        while new_size < min_size {
            new_size *= 2;
        }
        let mut new_buff = Buffer::new(
            new_size,
            usage,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            true,
            align_of::<T>(),
        )
        .context("Buffer creation failed")?;
        let new_data = new_buff.data().expect("Buffer should be mapped");
        let old_data = buffer.data().expect("Buffer should be mapped");
        new_data[..old_data.len()].copy_from_slice(old_data);

        *buffer = new_buff;

        Ok(())
    }

    unsafe fn get_buff_data<T>(buffer: &mut Buffer) -> &mut [T] {
        let buff = buffer.data().expect("Buffer should be mapped");
        let (a, data, b) = unsafe { buff.align_to_mut::<T>() };
        assert_eq!(a.len(), 0);
        assert_eq!(b.len(), 0);
        data
    }

    fn fill_uniforms(&mut self, swapchain: &Swapchain) {
        let data = Vec2::new(
            swapchain.extent.width as f32,
            swapchain.extent.height as f32,
        );
        for uniform in &mut self.uniforms {
            uniform.write(data);
        }
    }

    #[inline]
    pub fn recreate(&mut self, swapchain: &Swapchain, render_pass: &RenderPass) -> Result<()> {
        self.fill_uniforms(swapchain);
        let pipeline_options =
            Self::pipeline_options(&[&self.uniforms.layout, &self.descriptor_layout])?;
        self.pipeline
            .recreate::<gui::Vertex>(swapchain, render_pass, &pipeline_options)
            .context("Pipeline recreation failed")?;
        if swapchain.image_views.len() != self.vertex_buffers.len() {
            self.vertex_buffers = (0..swapchain.image_views.len())
                .map(|_| Self::create_vertex_buff())
                .collect::<Result<Vec<_>>>()
                .context("Vertex buffers creation failed")?;

            self.index_buffers = (0..swapchain.image_views.len())
                .map(|_| Self::create_index_buff())
                .collect::<Result<Vec<_>>>()
                .context("Vertex buffers creation failed")?;

            self.command_pool
                .realloc_buffers(&mut self.command_buffers, swapchain.image_views.len(), true)
                .context("Command buffers reallocation failed")?;
        }
        Ok(())
    }

    pub fn load_textures(&mut self, textures_delta: egui::TexturesDelta) -> Result<()> {
        for (id, delta) in &textures_delta.set {
            self.load_texture(*id, delta)
                .with_context(|| format!("Failed to load texture {:?}", id))?;
        }
        Ok(())
    }

    pub fn load_texture(
        &mut self,
        id: egui::TextureId,
        delta: &egui::epaint::ImageDelta,
    ) -> Result<()> {
        assert!(
            delta.pos.is_none(),
            "Textures sub-region update not supported (yet)"
        );

        let pixels: Vec<u8> = match &delta.image {
            egui::ImageData::Color(image) => {
                assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );
                image
                    .pixels
                    .iter()
                    .flat_map(|color| color.to_array())
                    .collect()
            }
            egui::ImageData::Font(image) => image
                .srgba_pixels(None)
                .flat_map(|color| color.to_array())
                .collect(),
        };
        let mut staging_buff =
            StagingBuffer::new(pixels.len(), 1).context("Staging buffer creation failed")?;
        let data = unsafe { staging_buff.data() };

        data.copy_from_slice(&pixels);

        let texture_options = TextureCreationOptions {
            format: vk::Format::R8G8B8A8_UNORM,
            address_mode: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            anisotropy: false,
            ..Default::default()
        };
        let descriptor_set = self
            .descriptor_pool
            .alloc_set(&self.descriptor_layout)
            .context("Descriptor set alloc failed")?;
        let texture = Texture::new(
            &mut self.textures_command_buff,
            &staging_buff,
            vk::Extent3D {
                width: delta.image.width() as u32,
                height: delta.image.height() as u32,
                depth: 1,
            },
            0,
            descriptor_set,
            &texture_options,
        )
        .context("Texture creation failed")?;
        self.textures.insert(id, texture);

        Ok(())
    }

    pub fn render(
        &mut self,
        image_index: usize,
        primitives: &[egui::ClippedPrimitive],
        textures_delta: egui::TexturesDelta,
        inheritance_info: &vk::CommandBufferInheritanceInfo,
    ) -> Result<vk::CommandBuffer> {
        self.load_textures(textures_delta)
            .context("Textures loading failed")?;

        let mut vert_count = 0;
        let mut index_count = 0;
        for primitive in primitives {
            let mesh = match &primitive.primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh,
                _ => unimplemented!(),
            };
            vert_count += mesh.vertices.len();
            index_count += mesh.indices.len();
        }

        let vertex_buff = &mut self.vertex_buffers[image_index];
        let index_buff = &mut self.index_buffers[image_index];

        if vert_count * size_of::<gui::Vertex>() > vertex_buff.size() {
            Self::resize_buff::<gui::Vertex>(
                vertex_buff,
                vert_count,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            )
            .context("Buffer resize failed")?;
        }
        if index_count * size_of::<u32>() > index_buff.size() {
            Self::resize_buff::<u32>(index_buff, index_count, vk::BufferUsageFlags::INDEX_BUFFER)
                .context("Buffer resize failed")?;
        }

        let command_buff = &mut self.command_buffers[image_index];
        command_buff.begin_secondary(inheritance_info)?;
        unsafe {
            DEVICE.cmd_bind_pipeline(
                **command_buff,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline,
            );
            DEVICE.cmd_bind_descriptor_sets(
                **command_buff,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                &[*self.uniforms[image_index].descriptor_set],
                &[],
            );
        }

        let vertex_buffer = vertex_buff.buffer;
        let index_buffer = index_buff.buffer;

        let vertex_data = unsafe { Self::get_buff_data(vertex_buff) };
        let index_data = unsafe { Self::get_buff_data(index_buff) };

        let mut vert_i = 0;
        let mut index_i = 0;
        for egui::ClippedPrimitive {
            primitive,
            clip_rect,
        } in primitives
        {
            let mesh = match primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh,
                _ => unimplemented!(),
            };

            let indices = &mesh.indices;
            let vertices = &mesh.vertices;

            index_data[index_i..index_i + indices.len()].copy_from_slice(indices);
            vertex_data[vert_i..vert_i + vertices.len()].copy_from_slice(vertices);

            let texture = self
                .textures
                .get(&mesh.texture_id)
                .with_context(|| format!("Texture {:?} not loaded", mesh.texture_id))?;
            unsafe {
                DEVICE.cmd_bind_descriptor_sets(
                    **command_buff,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.layout,
                    1,
                    &[*texture.descriptor_set],
                    &[],
                );
                DEVICE.cmd_bind_vertex_buffers(**command_buff, 0, &[vertex_buffer], &[0]);
                DEVICE.cmd_bind_index_buffer(
                    **command_buff,
                    index_buffer,
                    0,
                    vk::IndexType::UINT32,
                );
                let scissor = vk::Rect2D {
                    offset: vk::Offset2D {
                        x: clip_rect.min.x as i32,
                        y: clip_rect.min.y as i32,
                    },
                    extent: vk::Extent2D {
                        width: clip_rect.width() as u32,
                        height: clip_rect.height() as u32,
                    },
                };
                DEVICE.cmd_set_scissor(**command_buff, 0, &[scissor]);
                DEVICE.cmd_draw_indexed(
                    **command_buff,
                    indices.len() as _,
                    1,
                    index_i as _,
                    vert_i as _,
                    0,
                );
            }

            index_i += indices.len();
            vert_i += vertices.len();
        }
        command_buff.end()?;

        Ok(**command_buff)
    }
}
