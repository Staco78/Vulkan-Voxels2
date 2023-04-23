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
#![feature(lazy_cell)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(let_chains)]

mod app;
mod debug;
mod events;
mod inputs;
mod options;
mod render;
mod utils;
mod world;

use anyhow::{Context, Result};
use app::App;
use log::LevelFilter;
use render::Window;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

fn main() -> Result<()> {
    init_logger()?;

    let (window, event_loop) = Window::new()?;

    let mut app = App::new(window, &event_loop)?;

    event_loop.run(move |event, _, control_flow| {
        let r = app.tick_event(event).expect("App ticking failed");
        if let Some(new_control_flow) = r {
            *control_flow = new_control_flow;
        }
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
