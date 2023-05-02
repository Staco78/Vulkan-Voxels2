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
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(let_chains)]
#![feature(test)]
#![feature(hash_drain_filter)]
#![feature(return_position_impl_trait_in_trait)]
#![feature(unsize)]
#![feature(int_roundings)]
#![feature(pointer_byte_offsets)]

extern crate test;

mod app;
mod debug;
mod events;
mod gui;
mod inputs;
mod options;
mod render;
mod utils;
mod world;

use anyhow::{Context, Result};
use app::App;
use log::LevelFilter;
use render::Window;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode, ThreadLogMode};

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
        .set_thread_mode(ThreadLogMode::Both)
        .add_filter_ignore_str("meshing")
        .add_filter_ignore_str("allocator")
        .build();
    TermLogger::init(
        LevelFilter::Trace,
        config,
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")
}

/// Run init code for the tests.
#[cfg(test)]
#[ctor::ctor]
fn init() {
    let (window, event_loop) = Window::new().expect("Window creation failed");
    window.set_visible(false);
    let _app = App::new(window, &event_loop).expect("App creation failed");
}
