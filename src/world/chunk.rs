use anyhow::{Context, Result};
use log::trace;
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::render::{devices::DEVICE, Buffer};

use super::{blocks::BlockId, pos::ChunkPos, BLOCKS_PER_CHUNK};

#[derive(Debug)]
pub struct Chunk {
    pub blocks: [BlockId; BLOCKS_PER_CHUNK],
    pub buffer: Option<Buffer>,
    pub descriptor_set: vk::DescriptorSet,
}

impl Chunk {
    pub fn generate(pos: ChunkPos, buffer: &Buffer) -> Result<Self> {
        trace!("Generate chunk {:?}", pos);

        let descriptor_layout = {
            let binding = vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::MESH_EXT);
            let bindings = &[binding];
            let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);
            unsafe { DEVICE.create_descriptor_set_layout(&info, None) }
                .context("Descriptor set layout creation failed")?
        };

        let descriptor_pool = {
            let ubo_size = vk::DescriptorPoolSize::builder()
                .type_(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1);

            let pool_sizes = &[ubo_size];
            let info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(pool_sizes)
                .max_sets(1);

            unsafe { DEVICE.create_descriptor_pool(&info, None) }
                .context("Descriptor pool creation failed")?
        };

        let descriptor_set = {
            let layouts = &[descriptor_layout];
            let info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(layouts);

            let sets = unsafe { DEVICE.allocate_descriptor_sets(&info) }
                .context("Descriptor sets alloc failed")?;

            let info = vk::DescriptorBufferInfo::builder()
                .buffer(buffer.buffer)
                .offset(0)
                .range(buffer.size() as u64);

            let buffer_info = &[info];
            let ubo_write = vk::WriteDescriptorSet::builder()
                .dst_set(sets[0])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(buffer_info);

            unsafe { DEVICE.update_descriptor_sets(&[ubo_write], &[] as &[vk::CopyDescriptorSet]) };

            sets[0]
        };

        let mut blocks = [BlockId::Air; BLOCKS_PER_CHUNK];
        for (i, block) in blocks.iter_mut().enumerate() {
            if i % 4 == 0 && i % 7 == 0 {
                *block = BlockId::Block;
            }
        }
        Ok(Self {
            blocks,
            buffer: None,
            descriptor_set,
        })
    }
}
