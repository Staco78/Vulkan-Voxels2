use std::time::Instant;

use anyhow::{Context, Result};
use log::warn;
use winit::{
    event::{
        DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
};

use crate::{
    debug,
    events::{self, MainLoopEvent},
    gui::GuiContext,
    inputs::Inputs,
    options::AppOptions,
    render::{Renderer, Window},
    world::World,
};

#[derive(Debug)]
pub struct App {
    game_focused: bool,
    window: Window,
    renderer: Renderer,
    world: World,
    inputs: Inputs,

    last_frame_time: Instant,

    gui: GuiContext,
}

impl App {
    pub fn new(window: Window, event_loop: &EventLoop<MainLoopEvent>) -> Result<Self> {
        events::init_proxy(event_loop);
        let renderer = Renderer::new(&window).context("Renderer creation failed")?;
        let inputs = Inputs::new();
        let mut s = Self {
            game_focused: true,
            window,
            renderer,
            world: World::new().context("World creation failed")?,
            inputs,
            last_frame_time: Instant::now(),
            gui: GuiContext::new(event_loop),
        };
        s.set_game_focused(true);
        Ok(s)
    }

    pub fn tick_event(&mut self, event: Event<MainLoopEvent>) -> Result<Option<ControlFlow>> {
        let control_flow = match event {
            Event::WindowEvent { event, .. } => {
                let propagate = self.gui.on_event(&event);
                if !propagate {
                    return Ok(None);
                }
                match event {
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
                            ElementState::Pressed => {
                                if key == VirtualKeyCode::Escape {
                                    self.set_game_focused(false);
                                }
                                debug::key_pressed(key);
                                self.inputs.key_pressed(key)
                            }
                            ElementState::Released => self.inputs.key_released(key),
                        }
                        None
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if state == ElementState::Pressed && button == MouseButton::Left {
                            self.set_game_focused(true);
                        }
                        None
                    }
                    WindowEvent::Focused(focused) => {
                        self.set_game_focused(focused);
                        None
                    }
                    _ => None,
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if self.game_focused {
                    self.inputs.mouse_moved(delta);
                }
                None
            }
            Event::MainEventsCleared => {
                let now = Instant::now();
                let elasped = now - self.last_frame_time;
                self.last_frame_time = now;

                if AppOptions::get().tick_world {
                    self.world
                        .tick(self.renderer.camera_pos())
                        .context("World ticking failed")?;
                }

                let gui_data = self.gui.render(&self.window);

                self.renderer
                    .render(
                        elasped,
                        &self.window,
                        &self.inputs,
                        &self.world,
                        &gui_data.0,
                        gui_data.1,
                    )
                    .context("Rendering failed")?;
                None
            }
            Event::UserEvent(event) => match event {
                MainLoopEvent::RecreatePipeline => {
                    self.renderer
                        .recreate_pipeline()
                        .context("Pipeline recreation failed")?;
                    None
                }
            },
            _ => None,
        };
        Ok(control_flow)
    }

    fn set_game_focused(&mut self, focused: bool) {
        self.game_focused = focused;
        if focused {
            self.window.grab_cursor();
            self.window.set_cursor_visible(false);
        } else {
            self.window.release_cursor();
            self.window.set_cursor_visible(true);
        }
    }
}
