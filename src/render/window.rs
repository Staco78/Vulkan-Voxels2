use anyhow::{Context, Result};
use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub fn create_window() -> Result<(Window, EventLoop<()>)> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkan Voxels 2")
        .build(&event_loop)
        .context("Window creation failed")?;
    Ok((window, event_loop))
}
