mod blocks;
pub mod chunk;
mod pos;

use anyhow::{Context, Result};
pub use pos::*;

use core::slice;
use std::{collections::HashMap, mem::size_of, ops::Deref, sync::RwLock};

use crate::render::{Buffer, Vertex};

use vulkanalia::vk;

use self::chunk::Chunk;

pub const CHUNK_SIZE: usize = 32;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
pub const MAX_VERTICES_PER_CHUNK: usize = BLOCKS_PER_CHUNK * 18;
pub const RENDER_DISTANCE: usize = 2;

#[derive(Debug)]
pub struct World {
    chunks: RwLock<HashMap<ChunkPos, Chunk>>,
}

impl World {
    pub fn new() -> World {
        Self {
            chunks: RwLock::new(HashMap::new()),
        }
    }

    pub fn tick(&self, player_pos: EntityPos) -> Result<()> {
        let player_chunk_pos = player_pos.chunk();
        let (px, py, pz) = player_chunk_pos.xyz();
        let mut chunks = self.chunks.write().expect("Lock poisoned");
        for x in (px - RENDER_DISTANCE as i64)..=(px + RENDER_DISTANCE as i64) {
            for y in (py - RENDER_DISTANCE as i64)..=(py + RENDER_DISTANCE as i64) {
                for z in (pz - RENDER_DISTANCE as i64)..=(pz + RENDER_DISTANCE as i64) {
                    let chunk_pos = ChunkPos::new(x, y, z);
                    if chunks.contains_key(&chunk_pos) {
                        continue;
                    }
                    let mut chunk = Chunk::generate(chunk_pos);
                    let mut buff =
                        Buffer::new(MAX_VERTICES_PER_CHUNK, vk::BufferUsageFlags::VERTEX_BUFFER)
                            .context("Buffer creation failed")?;
                    let data = buff.map().context("Buffer map failed")?;
                    let vertices = unsafe {
                        slice::from_raw_parts_mut(
                            data.as_ptr() as *mut Vertex,
                            data.len() / size_of::<Vertex>(),
                        )
                    };
                    chunk.mesh(vertices);
                    // Safety: `data` and `vertices` aren't reused after this call.
                    unsafe { buff.unmap() }.context("Buffer unmap failed")?;
                    chunk.vertex_buffer = Some(buff);
                    chunks.insert(chunk_pos, chunk);
                }
            }
        }

        Ok(())
    }

    #[inline(always)]
    pub fn chunks(&self) -> impl Deref<Target = HashMap<ChunkPos, Chunk>> + '_ {
        self.chunks.read().expect("Lock poisoned")
    }
}
