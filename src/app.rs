use std::time::Instant;

use anyhow::{Context, Result};
use log::warn;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, WindowEvent},
    event_loop::ControlFlow,
};

use crate::{
    inputs::Inputs,
    render::{Renderer, Window},
};

#[derive(Debug)]
pub struct App {
    window: Window,
    renderer: Renderer,
    inputs: Inputs,

    last_frame_time: Instant,
}

impl App {
    pub fn new(window: Window) -> Result<Self> {
        let renderer = Renderer::new(&window).context("Renderer creation failed")?;
        let inputs = Inputs::new();
        window.grab_cursor();
        window.set_cursor_visible(false);
        Ok(Self {
            window,
            renderer,
            inputs,
            last_frame_time: Instant::now(),
        })
    }

    pub fn tick_event(&mut self, event: Event<()>) -> Result<Option<ControlFlow>> {
        let control_flow = match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => Some(ControlFlow::Exit),
                WindowEvent::Resized(_) => {
                    self.renderer
                        .recreate_swapchain(&self.window)
                        .context("Swapchain recreation failed")?;
                    None
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    let KeyboardInput {
                        state,
                        virtual_keycode,
                        scancode,
                        ..
                    } = input;
                    let key = if let Some(keycode) = virtual_keycode {
                        keycode
                    } else {
                        warn!("Unknown key: {}", scancode);
                        return Ok(None);
                    };
                    match state {
                        ElementState::Pressed => self.inputs.key_pressed(key),
                        ElementState::Released => self.inputs.key_released(key),
                    }
                    None
                }
                WindowEvent::Focused(focused) => {
                    if focused {
                        self.window.grab_cursor();
                        self.window.set_cursor_visible(false);
                    } else {
                        self.window.release_cursor();
                        self.window.set_cursor_visible(true);
                    }
                    None
                }
                _ => None,
            },
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                self.inputs.mouse_moved(delta);
                None
            }
            Event::MainEventsCleared => {
                let now = Instant::now();
                let elasped = now - self.last_frame_time;
                self.last_frame_time = now;

                self.renderer
                    .render(elasped, &self.window, &self.inputs)
                    .context("Rendering failed")?;
                None
            }

            _ => None,
        };
        Ok(control_flow)
    }
}
