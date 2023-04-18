use winit::event_loop::{EventLoop, EventLoopProxy};

use crate::utils::DerefOnceCell;

thread_local! {
    static EVENT_LOOP_PROXY: DerefOnceCell<EventLoopProxy<MainLoopEvent>, "Event loop proxy not initialized yet"> = DerefOnceCell::new();
}

pub fn init_proxy(event_loop: &EventLoop<MainLoopEvent>) {
    let proxy = event_loop.create_proxy();
    EVENT_LOOP_PROXY.with(|cell| {
        cell.inner()
            .set(proxy)
            .expect("Event loop proxy already initialized")
    })
}

#[inline(always)]
pub fn send_event(event: MainLoopEvent) {
    EVENT_LOOP_PROXY
        .with(|cell| cell.send_event(event))
        .expect("Event loop destroyed")
}

/// Event type that can be sent to the main loop.
#[derive(Debug)]
pub enum MainLoopEvent {
    RecreatePipeline,
}
