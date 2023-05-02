use std::{marker::Unsize, mem::size_of};

use memoffset::offset_of;
use vulkanalia::vk::{self, HasBuilder};

pub trait VertexDescriptor {
    fn binding_description() -> vk::VertexInputBindingDescription;
    fn attribute_descriptions() -> impl Unsize<[vk::VertexInputAttributeDescription]>;
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Vertex {
    pub data: u32,
}

impl VertexDescriptor for Vertex {
    fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    fn attribute_descriptions() -> impl Unsize<[vk::VertexInputAttributeDescription]> {
        [vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32_UINT)
            .offset(offset_of!(Self, data) as u32)
            .build()]
    }
}
