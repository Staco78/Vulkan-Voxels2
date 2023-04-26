mod blocks;
pub mod chunk;
mod chunk_mesh;
mod chunks;
mod generator;
pub mod meshing;
mod pos;

pub use pos::*;

use anyhow::Result;

use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};

use self::chunks::Chunks;

pub const CHUNK_SIZE: usize = 32;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const MAX_VERTICES_PER_CHUNK: usize = BLOCKS_PER_CHUNK * 18;
pub const RENDER_DISTANCE: usize = 5;

#[derive(Debug)]
pub struct World {
    chunks: Arc<RwLock<Chunks>>,
}

impl World {
    pub fn new() -> Result<World> {
        Ok(Self {
            chunks: Chunks::new(),
        })
    }

    pub fn tick(&self, player_pos: EntityPos) -> Result<()> {
        let player_chunk_pos = player_pos.chunk();
        let (px, py, pz) = player_chunk_pos.xyz();
        let mut chunks = self.chunks.write().expect("Lock poisoned");
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

    #[inline(always)]
    pub fn chunks(&self) -> impl Deref<Target = Chunks> + '_ {
        self.chunks.read().expect("Lock poisoned")
    }
}
