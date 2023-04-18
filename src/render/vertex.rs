use std::mem::size_of;

use memoffset::offset_of;
use nalgebra_glm::TVec3;
use vulkanalia::vk::{self, HasBuilder};

#[derive(Debug)]
#[repr(C)]
pub struct Vertex {
    pub pos: TVec3<u8>,
}

impl Vertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 1] {
        [vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R8G8B8_UINT)
            .offset(offset_of!(Self, pos) as u32)
            .build()]
    }
}
