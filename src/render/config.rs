use std::ffi::c_char;

use vulkanalia::vk::{Extension, KHR_SWAPCHAIN_EXTENSION};

pub const VALIDATION_ENABLED: bool = cfg!(debug_assertions);
pub const VALIDATION_LAYERS: &[*const c_char] = &[b"VK_LAYER_KHRONOS_validation\0".as_ptr().cast()];

pub const DEVICE_REQUIRED_EXTENSIONS: &[Extension] = &[KHR_SWAPCHAIN_EXTENSION];
