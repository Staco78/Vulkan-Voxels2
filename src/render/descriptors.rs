use std::{ops::Deref, slice};

use anyhow::{Context, Result};
use vulkanalia::vk::{self, DeviceV1_0, HasBuilder};

use crate::render::DEVICE;

#[derive(Debug)]
pub struct DescriptorSetLayout {
    inner: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    pub fn new(binding: &impl vk::Cast<Target = vk::DescriptorSetLayoutBinding>) -> Result<Self> {
        let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(slice::from_ref(binding));
        let layout = unsafe { DEVICE.create_descriptor_set_layout(&info, None) }
            .context("Layout creation failed")?;

        Ok(Self { inner: layout })
    }
}

impl Deref for DescriptorSetLayout {
    type Target = vk::DescriptorSetLayout;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe { DEVICE.destroy_descriptor_set_layout(self.inner, None) };
    }
}

#[derive(Debug)]
pub struct DescriptorPool {
    inner: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(max_sets: usize, descriptors_type: vk::DescriptorType) -> Result<Self> {
        let pool_size = vk::DescriptorPoolSize::builder()
            .descriptor_count(max_sets as u32)
            .type_(descriptors_type);
        let pool_sizes = &[pool_size];
        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(pool_sizes)
            .max_sets(max_sets as u32);

        let pool = unsafe { DEVICE.create_descriptor_pool(&info, None) }
            .context("Descriptor pool creation failed")?;

        Ok(Self { inner: pool })
    }

    pub fn alloc_sets(
        &mut self,
        count: usize,
        layout: &DescriptorSetLayout,
    ) -> Result<Vec<DescriptorSet>> {
        let layouts = vec![layout.inner; count];
        let info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.inner)
            .set_layouts(&layouts);
        let sets =
            unsafe { DEVICE.allocate_descriptor_sets(&info) }.context("Allocation failed")?;
        Ok(sets.iter().map(|&set| DescriptorSet::new(set)).collect())
    }

    pub fn alloc_set(&mut self, layout: &DescriptorSetLayout) -> Result<DescriptorSet> {
        let sets = self.alloc_sets(1, layout)?;
        let set = sets.into_iter().next().expect("Should contain one set");
        Ok(set)
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe { DEVICE.destroy_descriptor_pool(self.inner, None) };
    }
}

#[derive(Debug)]
pub struct DescriptorSet {
    inner: vk::DescriptorSet,
}

impl DescriptorSet {
    fn new(set: vk::DescriptorSet) -> Self {
        Self { inner: set }
    }

    #[inline]
    pub fn update(&mut self, descriptor_writes: &[impl vk::Cast<Target = vk::WriteDescriptorSet>]) {
        unsafe { DEVICE.update_descriptor_sets(descriptor_writes, &[] as &[vk::CopyDescriptorSet]) }
    }
}

impl Deref for DescriptorSet {
    type Target = vk::DescriptorSet;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
