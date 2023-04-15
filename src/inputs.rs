use std::{
    collections::HashSet,
    fmt::Debug,
    ptr,
    sync::atomic::{AtomicU64, Ordering},
};

use winit::event::VirtualKeyCode;

#[derive(Debug, Default)]
struct Delta(AtomicU64, AtomicU64);

impl Delta {
    #[inline]
    fn add(&self, delta: (f64, f64)) {
        fn update(val: u64, delta: f64) -> Option<u64> {
            union Union {
                raw: u64,
                val: f64,
            }
            let mut val = Union { raw: val };
            unsafe {
                val.val += delta;
                Some(val.raw)
            }
        }
        // Ignore errors because we always return `Some(_)`.
        let _ = self
            .0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                update(val, delta.0)
            });
        let _ = self
            .1
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                update(val, delta.1)
            });
    }
    #[inline]
    fn fetch_reset(&self) -> (f64, f64) {
        fn update(_: u64) -> Option<u64> {
            let val = unsafe {
                let val = 0.0_f64;
                ptr::read(&val as *const _ as *const u64)
            };
            Some(val)
        }

        #[allow(clippy::unwrap_used)]
        let (a, b) = {
            let a = self
                .0
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, update)
                .unwrap();
            let b = self
                .1
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, update)
                .unwrap();
            (a, b)
        };
        unsafe {
            let a = ptr::read(&a as *const _ as *const f64);
            let b = ptr::read(&b as *const _ as *const f64);
            (a, b)
        }
    }
}

#[derive(Debug)]
pub struct Inputs {
    keys: HashSet<VirtualKeyCode>,
    mouse_delta: Delta,
}

impl Inputs {
    pub fn new() -> Self {
        Self {
            keys: HashSet::new(),
            mouse_delta: Default::default(),
        }
    }

    #[inline(always)]
    pub fn key_pressed(&mut self, key: VirtualKeyCode) {
        self.keys.insert(key);
    }

    #[inline(always)]
    pub fn key_released(&mut self, key: VirtualKeyCode) {
        self.keys.remove(&key);
    }

    #[inline(always)]
    pub fn mouse_moved(&mut self, delta: (f64, f64)) {
        self.mouse_delta.add(delta);
    }

    /// Fetch the mouse delta and reset it.
    /// Should be called only once per frame.
    #[inline(always)]
    pub fn fetch_mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta.fetch_reset()
    }

    #[inline(always)]
    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys.contains(&key)
    }
}
