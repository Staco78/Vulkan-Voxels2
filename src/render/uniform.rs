use std::{
    mem::{align_of, size_of},
    ops::{Index, IndexMut},
    ptr, slice,
};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, HasBuilder};

use super::{
    buffer::Buffer,
    descriptors::{DescriptorPool, DescriptorSet, DescriptorSetLayout},
    devices::DEVICE,
};

#[derive(Debug)]
pub struct Uniforms<T> {
    _pool: DescriptorPool,
    pub layout: DescriptorSetLayout,
    uniforms: Vec<Uniform<T>>,
    _buff: Buffer,
}

impl<T> Uniforms<T> {
    pub fn new(count: usize) -> Result<Self> {
        let entry_size = size_of::<T>()
            .max(DEVICE.properties.limits.min_uniform_buffer_offset_alignment as usize);
        let entry_align = align_of::<T>()
            .max(DEVICE.properties.limits.min_uniform_buffer_offset_alignment as usize);
        let mut buff = Buffer::new(
            entry_size * count,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE,
            true,
            entry_align,
        )
        .context("Buffer creation failed")?;

        let mut pool = DescriptorPool::new(count, vk::DescriptorType::UNIFORM_BUFFER)
            .context("Descriptor pool creation failed")?;

        let layout = DescriptorSetLayout::new(&Self::binding(0))
            .context("Descriptor set layout creation failed")?;

        let sets = pool
            .alloc_sets(count, &layout)
            .context("Descriptor sets allocation failed")?;

        let ptr = buff.data().expect("Buffer should be mapped").as_ptr() as usize;
        let mut off = 0;
        let uniforms = sets
            .into_iter()
            .map(|mut set| {
                let buff_info = vk::DescriptorBufferInfo::builder()
                    .buffer(buff.buffer)
                    .offset(off as u64)
                    .range(size_of::<T>() as u64)
                    .build();

                let write = vk::WriteDescriptorSet::builder()
                    .dst_set(*set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(slice::from_ref(&buff_info));
                set.update(&[write]);

                let ptr = (ptr + off) as *mut T;
                off += entry_size;
                Uniform {
                    descriptor_set: set,
                    ptr,
                }
            })
            .collect();

        Ok(Self {
            _pool: pool,
            layout,
            uniforms,
            _buff: buff,
        })
    }

    pub fn binding(binding: u32) -> vk::DescriptorSetLayoutBindingBuilder<'static> {
        vk::DescriptorSetLayoutBinding::builder()
            .binding(binding)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.uniforms.len()
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

impl<'a, T> IntoIterator for &'a Uniforms<T> {
    type Item = &'a Uniform<T>;
    type IntoIter = slice::Iter<'a, Uniform<T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.uniforms.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Uniforms<T> {
    type Item = &'a mut Uniform<T>;
    type IntoIter = slice::IterMut<'a, Uniform<T>>;
    fn into_iter(self) -> Self::IntoIter {
        self.uniforms.iter_mut()
    }
}

#[derive(Debug)]
pub struct Uniform<T> {
    pub descriptor_set: DescriptorSet,
    ptr: *mut T,
}

impl<T> Uniform<T> {
    #[inline(always)]
    pub fn write(&mut self, val: T) {
        // Safety: this struct is created with a valid ptr.
        unsafe { ptr::write(self.ptr, val) }
    }
}
