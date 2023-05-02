use std::{fmt::Debug, time::Instant};

use egui::{ClippedPrimitive, TexturesDelta, Ui};
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget, window::Window};

pub type Vertex = egui::epaint::Vertex;

pub struct GuiContext {
    ctx: egui::Context,
    state: egui_winit::State,

    fps_counter: FpsCalculator,
}

impl Debug for GuiContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuiContext")
            .field("inner", &self.ctx)
            .finish_non_exhaustive()
    }
}

impl GuiContext {
    pub fn new<T>(event_loop: &EventLoopWindowTarget<T>) -> Self {
        let ctx = egui::Context::default();
        let state = egui_winit::State::new(event_loop);
        Self {
            ctx,
            state,
            fps_counter: FpsCalculator::new(),
        }
    }

    /// Return `true` if the event should be propagated.
    pub fn on_event(&mut self, event: &WindowEvent) -> bool {
        let response = self.state.on_event(&self.ctx, event);
        !response.consumed
    }

    pub fn render(&mut self, window: &Window) -> (Vec<ClippedPrimitive>, TexturesDelta) {
        self.fps_counter.tick();
        let output = self.ctx.run(self.state.take_egui_input(window), |ctx| {
            egui::Window::new("Debug")
                .resizable(false)
                .movable(false)
                .show(ctx, |ui| self.ui(ui));
        });
        self.state
            .handle_platform_output(window, &self.ctx, output.platform_output);
        (self.ctx.tessellate(output.shapes), output.textures_delta)
    }

    fn ui(&self, ui: &mut Ui) {
        ui.label(format!("FPS: {:.2}", self.fps_counter.fps));
    }
}

#[derive(Debug)]
struct FpsCalculator {
    fps: f32,
    start_time: Instant,
    frame_count: u32,
}

impl FpsCalculator {
    fn new() -> Self {
        Self {
            fps: 0.,
            start_time: Instant::now(),
            frame_count: 0,
        }
    }

    fn tick(&mut self) {
        const FRAMES_COUNT: u32 = 30;

        self.frame_count += 1;
        if self.frame_count == FRAMES_COUNT {
            let now = Instant::now();
            let elapsed = now - self.start_time;
            self.fps = FRAMES_COUNT as f32 / elapsed.as_secs_f32();
            self.start_time = now;
            self.frame_count = 0;
        }
    }
}
