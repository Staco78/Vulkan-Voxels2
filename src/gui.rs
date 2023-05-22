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
        let mut data = DATA.write().expect("Lock poisoned");
        data.fps_calculator.tick();

        let output = self.ctx.run(self.state.take_egui_input(window), |ctx| {
            egui::Window::new("Debug")
                .resizable(false)
                .movable(false)
                .show(ctx, |ui| self.ui(ui, &mut data));
        });

        #[cfg(feature = "bench")]
        crate::bench::append(&data);
        data.reset();

        self.state
            .handle_platform_output(window, &self.ctx, output.platform_output);
        (self.ctx.tessellate(output.shapes), output.textures_delta)
    }

    fn ui(&self, ui: &mut Ui, data: &mut Data) {
        ui.label(format!("Fps: {:.2}", data.fps_calculator.fps()));
        ui.label(format!(
            "Frame time: {:.2?}",
            data.fps_calculator.frame_time
        ));
        ui.label(format!("Position: {}", data.camera_pos));
        let chunk_pos = data.camera_pos.chunk();
        ui.label(format!("Chunk: {}", chunk_pos));
        ui.label(format!("Region: {}", chunk_pos.region()));
        ui.label(format!(
            "Chunks created/generated/meshed: {}/{}/{}",
            data.created_chunks_total.load(Ordering::Relaxed),
            data.generated_chunks_total.load(Ordering::Relaxed),
            data.meshed_chunks_total.load(Ordering::Relaxed)
        ));
        ui.label(format!(
            "Waiting for generation/meshing chunks: {}/{}",
            data.waiting_for_generate_chunks.load(Ordering::Relaxed),
            data.waiting_for_mesh_chunks.load(Ordering::Relaxed)
        ));
        ui.label(format!(
            "Loaded chunks/regions: {}/{}",
            data.loaded_chunks.load(Ordering::Relaxed),
            data.loaded_regions.load(Ordering::Relaxed)
        ));
    }
}

#[derive(Debug)]
pub struct Data {
    pub camera_pos: EntityPos,
    pub fps_calculator: FpsCalculator,

    pub created_chunks_total: AtomicUsize,
    pub generated_chunks_total: AtomicUsize,
    pub meshed_chunks_total: AtomicUsize,
    pub created_chunks: AtomicUsize,
    pub generated_chunks: AtomicUsize,
    pub meshed_chunks: AtomicUsize,

    pub waiting_for_generate_chunks: AtomicUsize,
    pub waiting_for_mesh_chunks: AtomicUsize,

    pub loaded_chunks: AtomicUsize,
    pub loaded_regions: AtomicUsize,
}

impl Data {
    const fn new() -> Self {
        Self {
            camera_pos: EntityPos::new(0., 0., 0., 0., 0.),
            fps_calculator: FpsCalculator::new(),

            created_chunks_total: AtomicUsize::new(0),
            generated_chunks_total: AtomicUsize::new(0),
            meshed_chunks_total: AtomicUsize::new(0),
            created_chunks: AtomicUsize::new(0),
            generated_chunks: AtomicUsize::new(0),
            meshed_chunks: AtomicUsize::new(0),

            waiting_for_generate_chunks: AtomicUsize::new(0),
            waiting_for_mesh_chunks: AtomicUsize::new(0),

            loaded_chunks: AtomicUsize::new(0),
            loaded_regions: AtomicUsize::new(0),
        }
    }

    fn reset(&mut self) {
        self.created_chunks.store(0, Ordering::Relaxed);
        self.generated_chunks.store(0, Ordering::Relaxed);
        self.meshed_chunks.store(0, Ordering::Relaxed);
    }
}

pub static DATA: RwLock<Data> = RwLock::new(Data::new());

#[derive(Debug, Clone)]
pub struct FpsCalculator {
    pub frame_time: Duration,
    pub start_time: Option<Instant>,
    pub frame_count: u32,
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
        let start_time = *self.start_time.get_or_insert_with(Instant::now);

        self.frame_count += 1;
        if self.frame_count == FRAMES_COUNT {
            let now = Instant::now();
            let elapsed = now - start_time;
            self.frame_time = elapsed.div_f32(FRAMES_COUNT as f32);
            self.start_time = Some(now);
            self.frame_count = 0;
        }
    }

    pub fn fps(&self) -> f32 {
        if self.frame_time != Duration::ZERO {
            1. / self.frame_time.as_secs_f32()
        } else {
            1.
        }
    }
}
