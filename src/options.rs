use std::{ops::Deref, sync::RwLock};

use vulkanalia::vk;

pub static OPTIONS: RwLock<AppOptions> = RwLock::new(AppOptions::new());

#[derive(Debug)]
pub struct AppOptions {
    pub polygon_mode: vk::PolygonMode,
    pub tick_world: bool,
}

impl AppOptions {
    pub const fn new() -> Self {
        Self {
            polygon_mode: vk::PolygonMode::FILL,
            tick_world: true,
        }
    }

    #[inline]
    pub fn get() -> impl Deref<Target = Self> {
        OPTIONS.read().expect("Lock poisoned")
    }
}
