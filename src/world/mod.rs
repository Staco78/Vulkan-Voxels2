mod blocks;
pub mod chunk;
mod chunk_mesh;
pub mod chunks;
mod generator;
pub mod meshing;
mod pos;

pub use pos::*;

use anyhow::Result;

use std::sync::{Arc, RwLock};

use crate::render::RegionsManager;

use self::chunks::Chunks;

pub const CHUNK_SIZE: usize = 32;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const MAX_VERTICES_PER_CHUNK: usize = BLOCKS_PER_CHUNK * 18;
pub const RENDER_DISTANCE: usize = 10;
pub const DISCARD_DISTANCE: usize = RENDER_DISTANCE + 2;
pub const REGION_SIZE: usize = 8;

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

        for x in (px - RENDER_DISTANCE as i64)..=(px + RENDER_DISTANCE as i64) {
            for y in (py - RENDER_DISTANCE as i64)..=(py + RENDER_DISTANCE as i64) {
                for z in (pz - RENDER_DISTANCE as i64)..=(pz + RENDER_DISTANCE as i64) {
                    if y > 10 {
                        continue;
                    }
                    let chunk_pos = ChunkPos::new(x, y, z);
                    chunks.load(chunk_pos)?;
                }
            }
        }

        Ok(())
    }
}
