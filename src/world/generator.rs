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
use mini_moka::sync::Cache;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::{gui, world::LocalBlockPos};

use super::{
    blocks::BlockId, chunk::Chunk, chunks::Chunks, ChunkPos, FlatChunkPos, BLOCKS_PER_CHUNK,
    CHUNK_SIZE,
};

pub const THREADS_COUNT: usize = 2;
const MAX_HEIGHT_MAPS_CACHE: usize = 4096;

pub type Message = Weak<Chunk>;

static EXIT: AtomicBool = AtomicBool::new(false);
static HANDLES: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

pub fn create_sender() -> (Sender<Message>, Receiver<Message>) {
    crossbeam_channel::unbounded()
}

pub fn start_threads(seed: u32, receiver: Receiver<Message>, chunks: &Arc<RwLock<Chunks>>) {
    let mut handles = HANDLES.lock().expect("Mutex poisoned");
    handles.reserve(THREADS_COUNT);

    let cache = Cache::new(MAX_HEIGHT_MAPS_CACHE as u64);

    for i in 0..THREADS_COUNT {
        let receiver = receiver.clone();
        let chunks = Arc::clone(chunks);
        let cache = cache.clone();
        let handle = thread::Builder::new()
            .name(format!("Generator {}", i))
            .spawn(move || {
                #[allow(clippy::unwrap_used)]
                thread_main(seed, receiver, chunks, cache).unwrap()
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

fn thread_main(
    seed: u32,
    receiver: Receiver<Message>,
    chunks: Arc<RwLock<Chunks>>,
    height_maps_cache: Cache<FlatChunkPos, HeightMap>,
) -> Result<()> {
    let generator = Generator::new(seed, height_maps_cache);

    while !EXIT.load(Ordering::Relaxed) {
        let chunk = receiver.recv().context("Channel disconnected")?;
        if let Some(chunk) = chunk.upgrade() {
            let mut blocks_lock = chunk.blocks.write().expect("Lock poisoned");
            let solid_blocks_count = generator.generate(&chunk.pos, &mut blocks_lock.data);
            blocks_lock.solid_blocks_count = solid_blocks_count;
            drop(blocks_lock);
            if solid_blocks_count == 0 {
                continue;
            }
            chunks
                .read()
                .expect("Lock poisoned")
                .chunk_generated(&chunk);
            gui::DATA
                .read()
                .expect("Lock poisoned")
                .generated_chunks
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    Ok(())
}

type HeightMap = [u32; CHUNK_SIZE * CHUNK_SIZE];

#[derive(Debug)]
struct Generator {
    noise: Fbm<Perlin>,
    height_maps_cache: Cache<FlatChunkPos, HeightMap>,
}

impl Generator {
    fn new(seed: u32, height_maps_cache: Cache<FlatChunkPos, HeightMap>) -> Self {
        Self {
            noise: Fbm::new(seed).set_frequency(0.001),
            height_maps_cache,
        }
    }

    /// Return the solid blocks count.
    fn generate(&self, pos: &ChunkPos, blocks: &mut [BlockId; BLOCKS_PER_CHUNK]) -> u32 {
        let map = self.get_height_map(&pos.flat());

        let chunk_floor = pos.y * CHUNK_SIZE as i64;

        let mut solid_blocks = 0;

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let mut y = 0;
                while y < CHUNK_SIZE && (chunk_floor + y as i64) < map[x * CHUNK_SIZE + z] as i64 {
                    let pos = LocalBlockPos::new(x as u8, y as u8, z as u8);
                    blocks[pos.to_index()] = BlockId::Block;

                    solid_blocks += 1;
                    y += 1;
                }
            }
        }

        solid_blocks
    }

    fn get_height_map(&self, pos: &FlatChunkPos) -> HeightMap {
        self.height_maps_cache.get(pos).unwrap_or_else(|| {
            let map = self.create_height_map(pos);
            self.height_maps_cache.insert(*pos, map);
            map
        })
    }

    fn create_height_map(&self, pos: &FlatChunkPos) -> HeightMap {
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

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use test::Bencher;

    use super::*;

    #[bench]
    fn generate(b: &mut Bencher) {
        let mut blocks = [BlockId::Air; BLOCKS_PER_CHUNK];
        let cache = Cache::new(MAX_HEIGHT_MAPS_CACHE as u64);
        let generator = Generator::new(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as u32,
            cache,
        );
        let mut x = (generator.noise.get([0., 0.]) * 100.) as i64;
        let mut y = (generator.noise.get([-12., 35.]) * 100.) as i64;
        let mut z = (generator.noise.get([81., -90.]) * 100.) as i64;
        b.iter(|| {
            generator.generate(&ChunkPos::new(x, y, z), &mut blocks);
            x += 1;
            y += 1;
            z += 1;
        })
    }
}
