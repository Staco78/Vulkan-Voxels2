use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    },
    time::{Duration, Instant},
};

use egui::{ClippedPrimitive, TexturesDelta, Ui};
use winit::{event::WindowEvent, event_loop::EventLoopWindowTarget, window::Window};

use crate::world::EntityPos;

pub type Vertex = egui::epaint::Vertex;

pub struct GuiContext {
    ctx: egui::Context,
    state: egui_winit::State,
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
        Self { ctx, state }
    }

    /// Return `true` if the event should be propagated.
    pub fn on_event(&mut self, event: &WindowEvent) -> bool {
        let response = self.state.on_event(&self.ctx, event);
        !response.consumed
    }

    pub fn render(&mut self, window: &Window) -> (Vec<ClippedPrimitive>, TexturesDelta) {
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
        let mut data = DATA.write().expect("Lock poisoned");
        data.fps_calculator.tick();

        ui.label(format!(
            "Fps: {:.2}",
            1. / data.fps_calculator.frame_time.as_secs_f32()
        ));
        ui.label(format!(
            "Frame time: {:.2?}",
            data.fps_calculator.frame_time
        ));
        ui.label(format!("Position: {}", data.camera_pos));
        ui.label(format!("Chunk pos: {}", data.camera_pos.chunk()));
        ui.label(format!(
            "Chunks created/generated/meshed: {}/{}/{}",
            data.created_chunks.load(Ordering::Relaxed),
            data.generated_chunks.load(Ordering::Relaxed),
            data.meshed_chunks.load(Ordering::Relaxed)
        ));
    }
}

#[derive(Debug)]
pub struct Data {
    pub camera_pos: EntityPos,
    fps_calculator: FpsCalculator,
    pub created_chunks: AtomicUsize,
    pub generated_chunks: AtomicUsize,
    pub meshed_chunks: AtomicUsize,
}

impl Data {
    const fn new() -> Self {
        Self {
            camera_pos: EntityPos::new(0., 0., 0., 0., 0.),
            fps_calculator: FpsCalculator::new(),
            created_chunks: AtomicUsize::new(0),
            generated_chunks: AtomicUsize::new(0),
            meshed_chunks: AtomicUsize::new(0),
        }
    }
}

pub static DATA: RwLock<Data> = RwLock::new(Data::new());

#[derive(Debug)]
struct FpsCalculator {
    frame_time: Duration,
    start_time: Option<Instant>,
    frame_count: u32,
}

impl FpsCalculator {
    const fn new() -> Self {
        Self {
            frame_time: Duration::new(0, 0),
            start_time: None,
            frame_count: 0,
        }
    }

    fn tick(&mut self) {
        const FRAMES_COUNT: u32 = 5;
        let start_time = self.start_time.unwrap_or_else(Instant::now);

        self.frame_count += 1;
        if self.frame_count == FRAMES_COUNT {
            let now = Instant::now();
            let elapsed = now - start_time;
            self.frame_time = elapsed.div_f32(FRAMES_COUNT as f32);
            self.start_time = Some(now);
            self.frame_count = 0;
        }
    }
}
