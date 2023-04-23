use std::ops::Deref;

use anyhow::{Context, Result};
use log::warn;
use winit::{
    event_loop::{EventLoop, EventLoopBuilder},
    window::{CursorGrabMode, WindowBuilder},
};

use crate::events::MainLoopEvent;

#[derive(Debug)]
pub struct Window {
    window: winit::window::Window,
}

impl Deref for Window {
    type Target = winit::window::Window;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl Window {
    pub fn new() -> Result<(Self, EventLoop<MainLoopEvent>)> {
        let event_loop = EventLoopBuilder::with_user_event().build();
        let window = WindowBuilder::new()
            .with_title("Vulkan Voxels 2")
            .build(&event_loop)
            .context("Window creation failed")?;
        Ok((Self { window }, event_loop))
    }

    pub fn grab_cursor(&self) {
        // self.set_cursor_grab(CursorGrabMode::Confined)
        //     .or_else(|e| self.set_cursor_grab(CursorGrabMode::Locked).context(e))
        //     .unwrap_or_else(|_| warn!("Cursor grabbing failed"))
    }

    pub fn release_cursor(&self) {
        self.set_cursor_grab(CursorGrabMode::None)
            .unwrap_or_else(|_| warn!("Cursor release failed"))
    }
}
