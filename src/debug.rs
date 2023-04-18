use vulkanalia::vk;
use winit::event::VirtualKeyCode;

use crate::{
    events::{self, MainLoopEvent},
    options::OPTIONS,
};

pub fn key_pressed(key: VirtualKeyCode) {
    let event_to_send = match key {
        VirtualKeyCode::F1 => {
            let mut options = OPTIONS.write().expect("Lock poisoned");
            options.tick_world = !options.tick_world;
            None
        }
        VirtualKeyCode::F2 => {
            let mut options = OPTIONS.write().expect("Lock poisoned");
            if options.polygon_mode == vk::PolygonMode::FILL {
                options.polygon_mode = vk::PolygonMode::LINE;
            } else {
                options.polygon_mode = vk::PolygonMode::FILL
            }
            Some(MainLoopEvent::RecreatePipeline)
        }
        _ => None,
    };
    if let Some(event) = event_to_send {
        events::send_event(event)
    }
}
