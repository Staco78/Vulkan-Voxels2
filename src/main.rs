#![deny(
    clippy::correctness,
    clippy::suspicious,
    clippy::style,
    clippy::complexity,
    clippy::perf
)]
#![deny(
    clippy::empty_structs_with_brackets,
    clippy::clone_on_ref_ptr,
    clippy::dbg_macro,
    clippy::unwrap_used
)]

use ash::{vk, Entry};
use log::{debug, LevelFilter};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};

fn main() {
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");

    let entry = Entry::linked();
    let app_info = vk::ApplicationInfo {
        api_version: vk::make_api_version(0, 1, 0, 0),
        ..Default::default()
    };
    let create_info = vk::InstanceCreateInfo {
        p_application_info: &app_info,
        ..Default::default()
    };
    let instance = unsafe {
        entry
            .create_instance(&create_info, None)
            .expect("Instance creation fialed")
    };
    let devices = unsafe {
        instance
            .enumerate_physical_devices()
            .expect("Physical device enumeration failed")
    };
    let devices = devices
        .iter()
        .map(|d| unsafe { instance.get_physical_device_properties(Clone::clone(d)) });
    debug!("{:#?}", devices.collect::<Vec<_>>());
}
