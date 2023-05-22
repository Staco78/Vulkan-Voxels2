mod blocks;
pub mod chunk;
mod chunk_mesh;
pub mod chunks;
mod generator;
pub mod meshing;
mod pos;

pub use pos::*;

use anyhow::Result;

use std::sync::{atomic::Ordering, Arc, RwLock};

use crate::{gui, render::RegionsManager};

use self::chunks::Chunks;

pub const CHUNK_SIZE: usize = 32;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const MAX_VERTICES_PER_CHUNK: usize = BLOCKS_PER_CHUNK * 18;
pub const RENDER_DISTANCE: usize = 10;
pub const DISCARD_DISTANCE: usize = RENDER_DISTANCE + 2;
pub const REGION_SIZE: usize = 8;
pub const MAX_LOADED_CHUNKS_PER_FRAME: usize = 1000;

#[derive(Debug)]
pub struct World {
    chunks: Arc<RwLock<Chunks>>,
    regions: Arc<RegionsManager>,
}

impl World {
    pub fn new(chunks: Arc<RwLock<Chunks>>, regions: Arc<RegionsManager>) -> Result<Self> {
        Chunks::init(&chunks, &regions);
        Ok(Self { chunks, regions })
    }

    pub fn create_chunks() -> Arc<RwLock<Chunks>> {
        Chunks::new()
    }

    pub fn tick(&self, player_pos: EntityPos) -> Result<()> {
        let player_chunk_pos = player_pos.chunk();
        let (px, py, pz) = player_chunk_pos.xyz();
        let mut chunks = self.chunks.write().expect("Lock poisoned");

        chunks.update_gui_data();

        #[cfg(not(feature = "bench_chunks"))]
        chunks.drain_filter(
            |pos, _| {
                let dx = (px - pos.x()).abs();
                let dy = (py - pos.y()).abs();
                let dz = (pz - pos.z()).abs();
                dx > DISCARD_DISTANCE as _
                    || dy > DISCARD_DISTANCE as _
                    || dz > DISCARD_DISTANCE as _
            },
            &self.regions,
        );

        let mut loaded_chunks = 0;

        'outer: for x in (px - RENDER_DISTANCE as i64)..=(px + RENDER_DISTANCE as i64) {
            for y in (py - RENDER_DISTANCE as i64)..=(py + RENDER_DISTANCE as i64) {
                for z in (pz - RENDER_DISTANCE as i64)..=(pz + RENDER_DISTANCE as i64) {
                    let chunk_pos = ChunkPos::new(x, y, z);
                    if chunks.load(chunk_pos)? {
                        loaded_chunks += 1;
                    }

                    if loaded_chunks > MAX_LOADED_CHUNKS_PER_FRAME {
                        break 'outer;
                    }
                }
            }
        }

        gui::DATA
            .read()
            .expect("Lock poisoned")
            .loaded_chunks
            .store(chunks.len(), Ordering::Relaxed);

        Ok(())
    }
}
