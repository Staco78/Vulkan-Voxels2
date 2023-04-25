use std::mem::size_of;

use memoffset::offset_of;
use vulkanalia::vk::{self, HasBuilder};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Vertex {
    pub data: u32,
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
            .format(vk::Format::R32_UINT)
            .offset(offset_of!(Self, data) as u32)
            .build()]
    }
}
