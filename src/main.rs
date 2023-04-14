#![warn(
    clippy::correctness,
    clippy::suspicious,
    clippy::style,
    clippy::complexity,
    clippy::perf
)]
#![warn(
    clippy::unwrap_used,
    clippy::clone_on_ref_ptr,
    clippy::empty_structs_with_brackets,
    clippy::dbg_macro,
    unused_features
)]
#![allow(incomplete_features)]
#![feature(new_uninit)]
#![feature(maybe_uninit_write_slice)]
#![feature(maybe_uninit_slice)]
#![feature(adt_const_params)]

mod render;
mod utils;

use anyhow::{Context, Result};
use log::LevelFilter;
use render::{create_window, Renderer};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() -> Result<()> {
    init_logger()?;

    let (window, event_loop) = create_window()?;

    let mut renderer = Renderer::new(&window).context("Renderer creation failed")?;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(_) =>
            {
                #[allow(clippy::unwrap_used)]
                renderer
                    .recreate_swapchain(&window)
                    .context("Swapchain recreation failed")
                    .unwrap()
            }
            _ => (),
        },
        Event::MainEventsCleared => {
            #[allow(clippy::unwrap_used)]
            renderer
                .render(&window)
                .context("Rendering failed")
                .unwrap();
        }

        _ => (),
    });
}

fn init_logger() -> Result<()> {
    let config = ConfigBuilder::new()
        .set_time_level(LevelFilter::Off)
        .build();
    TermLogger::init(
        LevelFilter::Trace,
        config,
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")
}
