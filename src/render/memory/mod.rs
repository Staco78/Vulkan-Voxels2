mod allocator;
pub use allocator::*;
use vulkanalia::vk;

use std::sync::OnceLock;

static ALLOCATOR: OnceLock<Allocator> = OnceLock::new();

#[inline(always)]
pub fn allocator() -> &'static Allocator {
    ALLOCATOR.get().expect("Allocator not initialized")
}

#[inline(always)]
pub fn init_allocator(physical_device: vk::PhysicalDevice) {
    ALLOCATOR.get_or_init(|| Allocator::new(physical_device));
}
