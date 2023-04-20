mod blocks;
pub mod chunk;
mod pos;

use anyhow::{Context, Result};
pub use pos::*;

use std::{collections::HashMap, mem::size_of, ops::Deref, ptr, sync::RwLock};

use crate::render::Buffer;

use vulkanalia::vk;

use self::{blocks::BlockId, chunk::Chunk};

pub const CHUNK_SIZE: usize = 32;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
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
                    if y > 10 {
                        continue;
                    }
                    let chunk_pos = ChunkPos::new(x, y, z);
                    if chunks.contains_key(&chunk_pos) {
                        continue;
                    }
                    let mut buff = Buffer::new(
                        BLOCKS_PER_CHUNK * size_of::<BlockId>(),
                        vk::BufferUsageFlags::STORAGE_BUFFER,
                    )
                    .context("Buffer creation failed")?;
                    let mut chunk = Chunk::generate(chunk_pos, &buff)?;
                    let data = buff.map().context("Buffer map failed")?;
                    unsafe {
                        ptr::copy_nonoverlapping(
                            chunk.blocks.as_ptr(),
                            data.as_ptr() as *mut BlockId,
                            BLOCKS_PER_CHUNK,
                        )
                    };
                    // Safety: `data` and `vertices` aren't reused after this call.
                    unsafe { buff.unmap() }.context("Buffer unmap failed")?;
                    chunk.buffer = Some(buff);
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
