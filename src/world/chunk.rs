use std::sync::{Arc, Mutex, RwLock};

use log::trace;

use crate::{
    render::{Buffer, Vertex},
    world::chunk_mesh::{mesh, ADDENDS},
};

use super::{blocks::BlockId, chunks::Chunks, pos::ChunkPos, BLOCKS_PER_CHUNK};

#[derive(Debug)]
pub struct Chunk {
    pub(super) pos: ChunkPos,
    pub(super) blocks: RwLock<Option<[BlockId; BLOCKS_PER_CHUNK]>>,
    pub vertex_buffer: Mutex<Option<Buffer>>,
}

impl Chunk {
    pub fn new(pos: ChunkPos) -> Self {
        Self {
            pos,
            blocks: RwLock::new(None),
            vertex_buffer: Mutex::new(None),
        }
    }

    /// Return the count of vertices generated.
    pub fn mesh(&self, chunks: &Arc<RwLock<Chunks>>, buff: &mut [Vertex]) -> usize {
        trace!("Mesh chunk {:?}", self.pos);

        let mut neighbours: [Option<Arc<Chunk>>; 6] = [None, None, None, None, None, None];
        let chunks = chunks.read().expect("Lock poisoned");
        for i in 0..6 {
            let addend = ADDENDS[i];
            let addend_pos = ChunkPos::new(addend.0 as _, addend.1 as _, addend.2 as _);
            let pos = self.pos + addend_pos;
            let neighbour = chunks.get(&pos);
            neighbours[i] = neighbour.cloned();
        }
        drop(chunks);

        let blocks = self.blocks.read().expect("Lock poisoned");
        let blocks = blocks
            .as_ref()
            .expect("Trying to mesh a non-generated chunk");

        mesh(blocks, &neighbours, buff)
    }
}
