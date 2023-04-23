use std::{
    mem::size_of,
    ops::{Index, IndexMut},
    ptr,
};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use super::{buffer::Buffer, devices::DEVICE};

#[derive(Debug)]
pub struct Uniforms<T> {
    uniforms: Vec<Uniform<T>>,
    _buff: Buffer,
    pub descriptor_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
}

impl<T> Uniforms<T> {
    pub fn new(count: usize) -> Result<Self> {
        let mut buff = Buffer::new(
            count * size_of::<T>(),
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE,
            true,
        )
        .context("Buffer creation failed")?;
        let ptr = buff.data()?.as_mut_ptr() as *mut T;

        let descriptor_layout = {
            let binding = vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX);
            let bindings = &[binding];
            let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);
            unsafe { DEVICE.create_descriptor_set_layout(&info, None) }
                .context("Descriptor set layout creation failed")?
        };

        let descriptor_pool = {
            let ubo_size = vk::DescriptorPoolSize::builder()
                .type_(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(count as u32);

            let pool_sizes = &[ubo_size];
            let info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(pool_sizes)
                .max_sets(count as u32);

            unsafe { DEVICE.create_descriptor_pool(&info, None) }
                .context("Descriptor pool creation failed")?
        };

        let descriptor_sets = {
            let layouts = vec![descriptor_layout; count];
            let info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&layouts);

            let sets = unsafe { DEVICE.allocate_descriptor_sets(&info) }
                .context("Descriptor sets alloc failed")?;

            for (i, &set) in sets.iter().enumerate() {
                let info = vk::DescriptorBufferInfo::builder()
                    .buffer(buff.buffer)
                    .offset((i * size_of::<T>()) as u64)
                    .range(size_of::<T>() as u64);

                let buffer_info = &[info];
                let ubo_write = vk::WriteDescriptorSet::builder()
                    .dst_set(set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(buffer_info);

                unsafe {
                    DEVICE.update_descriptor_sets(&[ubo_write], &[] as &[vk::CopyDescriptorSet])
                };
            }

            sets
        };
        let uniforms = descriptor_sets
            .iter()
            .copied()
            .enumerate()
            .map(|(i, descriptor_set)| Uniform {
                ptr: unsafe { ptr.add(i) },
                descriptor_set,
            })
            .collect();

        Ok(Self {
            uniforms,
            _buff: buff,
            descriptor_layout,
            descriptor_pool,
        })
    }
}

impl<T> Drop for Uniforms<T> {
    fn drop(&mut self) {
        unsafe {
            DEVICE.destroy_descriptor_set_layout(self.descriptor_layout, None);
            DEVICE.destroy_descriptor_pool(self.descriptor_pool, None);
        };
    }
}

impl<T> Index<usize> for Uniforms<T> {
    type Output = Uniform<T>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.uniforms[index]
    }
}
impl<T> IndexMut<usize> for Uniforms<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.uniforms[index]
    }
}

#[derive(Debug)]
pub struct Uniform<T> {
    pub descriptor_set: vk::DescriptorSet,
    ptr: *mut T,
}

impl<T> Uniform<T> {
    #[inline(always)]
    pub fn write(&mut self, val: T) {
        // Safety: this struct is created with a valid ptr.
        unsafe { ptr::write(self.ptr, val) }
    }
}
