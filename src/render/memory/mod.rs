mod allocator;
pub use allocator::*;
use vulkanalia::{vk, Device, Instance};

use std::sync::OnceLock;

static ALLOCATOR: OnceLock<Allocator> = OnceLock::new();

#[inline(always)]
pub fn allocator() -> &'static Allocator {
    ALLOCATOR.get().expect("Allocator not initialized")
}

#[inline(always)]
pub fn init_allocator(device: &Device, instance: &Instance, physical_device: vk::PhysicalDevice) {
    ALLOCATOR.get_or_init(|| Allocator::new(device, instance, physical_device));
}
