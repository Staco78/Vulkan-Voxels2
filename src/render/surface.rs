use std::ops::Deref;

use anyhow::{Context, Result};
use vulkanalia::{
    vk::{self, KhrSurfaceExtension},
    window::create_surface,
};
use winit::window::Window;

use super::instance::INSTANCE;

#[derive(Debug)]
pub struct Surface {
    surface: vk::SurfaceKHR,
}

impl Surface {
    pub fn new(window: &Window) -> Result<Self> {
        let surface = unsafe { create_surface(&INSTANCE, window, window) }
            .context("Surface creation failed")?;
        Ok(Self { surface })
    }
}

impl Deref for Surface {
    type Target = vk::SurfaceKHR;
    fn deref(&self) -> &Self::Target {
        &self.surface
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { INSTANCE.destroy_surface_khr(self.surface, None) };
    }
}
