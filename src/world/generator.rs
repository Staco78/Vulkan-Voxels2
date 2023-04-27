use std::{
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock, Weak,
    },
    thread::{self, JoinHandle},
};

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::world::LocalBlockPos;

use super::{
    blocks::BlockId, chunk::Chunk, chunks::Chunks, FlatChunkPos, BLOCKS_PER_CHUNK, CHUNK_SIZE,
};

pub const THREADS_COUNT: usize = 2;

pub type Message = Weak<Chunk>;

static EXIT: AtomicBool = AtomicBool::new(false);
static HANDLES: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

pub fn create_sender() -> (Sender<Message>, Receiver<Message>) {
    crossbeam_channel::unbounded()
}

pub fn start_threads(seed: u32, receiver: Receiver<Message>, chunks: &Arc<RwLock<Chunks>>) {
    let mut handles = HANDLES.lock().expect("Mutex poisoned");
    handles.reserve(THREADS_COUNT);
    for i in 0..THREADS_COUNT {
        let receiver = receiver.clone();
        let chunks = Arc::clone(chunks);
        let handle = thread::Builder::new()
            .name(format!("Generator {}", i))
            .spawn(move || {
                #[allow(clippy::unwrap_used)]
                thread_main(seed, receiver, chunks).unwrap()
            })
            .expect("Thread spawn failed");
        handles.push(handle);
    }
}

pub fn stop_threads(sender: &Sender<Message>) {
    EXIT.store(true, Ordering::Relaxed);
    let mut handles = HANDLES.lock().expect("Mutex poisoned");
    for _ in 0..handles.len() {
        let _ = sender.send(Weak::new());
    }
    for handle in handles.drain(..) {
        let _ = handle.join();
    }
}

fn thread_main(seed: u32, receiver: Receiver<Message>, chunks: Arc<RwLock<Chunks>>) -> Result<()> {
    let generator = Generator::new(seed);

    while !EXIT.load(Ordering::Relaxed) {
        let chunk = receiver.recv().context("Channel disconnected")?;
        if let Some(chunk) = chunk.upgrade() {
            let mut blocks = [BlockId::Air; BLOCKS_PER_CHUNK];
            let map = generator.create_height_map(chunk.pos.flat());

            let chunk_floor = chunk.pos.y * CHUNK_SIZE as i64;

            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let mut y = 0;
                    while y < CHUNK_SIZE as _
                        && (chunk_floor + y as i64) < map[x * CHUNK_SIZE + z] as i64
                    {
                        let pos = LocalBlockPos::new(x as u8, y, z as u8);
                        blocks[pos.to_index()] = BlockId::Block;
                        y += 1;
                    }
                }
            }

            let mut blocks_lock = chunk.blocks.write().expect("Lock poisoned");
            debug_assert!(blocks_lock.is_none());
            *blocks_lock = Some(blocks);
            drop(blocks_lock);

            chunks
                .read()
                .expect("Lock poisoned")
                .chunk_generated(&chunk);
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Generator {
    noise: Fbm<Perlin>,
}

impl Generator {
    fn new(seed: u32) -> Self {
        Self {
            noise: Fbm::new(seed).set_frequency(0.001),
        }
    }
    fn create_height_map(&self, pos: FlatChunkPos) -> [u32; CHUNK_SIZE * CHUNK_SIZE] {
        let mut map: [MaybeUninit<u32>; CHUNK_SIZE * CHUNK_SIZE] = MaybeUninit::uninit_array();
        let off = (
            (pos.x() * CHUNK_SIZE as i64) as f64,
            (pos.z() * CHUNK_SIZE as i64) as f64,
        );
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let val = self.noise.get([off.0 + x as f64, off.1 + z as f64]);
                // scale from [-1; 1] to [0; 1]
                let val = (val + 1.) / 2.;
                let val = (val * 100.) as u32 + 50;
                map[x * CHUNK_SIZE + z].write(val);
            }
        }
        // Safety: we wrote each value
        unsafe { MaybeUninit::array_assume_init(map) }
    }
}
