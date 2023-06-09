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

        let mut load = |x: i32, y: i32, z: i32| -> Result<()> {
            let pos = ChunkPos::new(
                player_chunk_pos.x() + x as i64,
                player_chunk_pos.y() + y as i64,
                player_chunk_pos.z() + z as i64,
            );
            chunks.load(pos)?;
            Ok(())
        };

        let n = RENDER_DISTANCE as i32 * 3;
        let m = RENDER_DISTANCE as i32;
        for distance in 0..n - 1 {
            for i in 0..=distance {
                let x = i;
                for j in 0..=distance - x {
                    let y = j;
                    let z = distance - (x + y);
                    if x <= m && y <= m && z <= m {
                        load(x, y, z)?;
                        if x != 0 {
                            load(-x, y, z)?;
                        }
                        if y != 0 {
                            load(x, -y, z)?;
                        }
                        if z != 0 {
                            load(x, y, -z)?;
                        }
                        if x != 0 && y != 0 {
                            load(-x, -y, z)?;
                        }
                        if x != 0 && z != 0 {
                            load(-x, y, -z)?;
                        }
                        if y != 0 && z != 0 {
                            load(x, -y, -z)?;
                        }
                        if x != 0 && y != 0 && z != 0 {
                            load(-x, -y, -z)?;
                        }
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

impl Drop for World {
    fn drop(&mut self) {
        self.chunks.read().expect("Lock poisoned").stop_threads();
    }
}
