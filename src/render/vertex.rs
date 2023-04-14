use std::mem::size_of;

use memoffset::offset_of;
use nalgebra_glm::Vec3;
use vulkanalia::vk::{self, HasBuilder};

#[derive(Debug)]
#[repr(C)]
pub struct Vertex {
    pos: Vec3,
    color: Vec3,
}

impl Vertex {
    #[inline(always)]
    pub const fn new(pos: Vec3, color: Vec3) -> Self {
        Self { pos, color }
    }
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Self, pos) as u32)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Self, color) as u32)
                .build(),
        ]
    }
}
