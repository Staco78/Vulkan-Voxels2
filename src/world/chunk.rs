use std::sync::{Arc, Mutex, RwLock};

use log::trace;

use crate::{
    render::{Buffer, Vertex},
    world::{LocalBlockPos, CHUNK_SIZE},
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
        use meshing_consts::*;

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

mod meshing_consts {
    use nalgebra_glm::TVec3;

    pub const FRONT: [TVec3<u8>; 6] = [
        TVec3::new(0, 0, 0),
        TVec3::new(0, 1, 0),
        TVec3::new(0, 0, 1),
        TVec3::new(0, 1, 0),
        TVec3::new(0, 1, 1),
        TVec3::new(0, 0, 1),
    ];
    pub const BACK: [TVec3<u8>; 6] = [
        TVec3::new(1, 0, 0),
        TVec3::new(1, 0, 1),
        TVec3::new(1, 1, 0),
        TVec3::new(1, 1, 0),
        TVec3::new(1, 0, 1),
        TVec3::new(1, 1, 1),
    ];
    pub const LEFT: [TVec3<u8>; 6] = [
        TVec3::new(1, 0, 0),
        TVec3::new(1, 1, 0),
        TVec3::new(0, 0, 0),
        TVec3::new(1, 1, 0),
        TVec3::new(0, 1, 0),
        TVec3::new(0, 0, 0),
    ];
    pub const RIGHT: [TVec3<u8>; 6] = [
        TVec3::new(0, 0, 1),
        TVec3::new(0, 1, 1),
        TVec3::new(1, 0, 1),
        TVec3::new(0, 1, 1),
        TVec3::new(1, 1, 1),
        TVec3::new(1, 0, 1),
    ];
    pub const UP: [TVec3<u8>; 6] = [
        TVec3::new(0, 1, 0),
        TVec3::new(1, 1, 0),
        TVec3::new(0, 1, 1),
        TVec3::new(1, 1, 0),
        TVec3::new(1, 1, 1),
        TVec3::new(0, 1, 1),
    ];
    pub const DOWN: [TVec3<u8>; 6] = [
        TVec3::new(1, 0, 0),
        TVec3::new(0, 0, 0),
        TVec3::new(1, 0, 1),
        TVec3::new(0, 0, 0),
        TVec3::new(0, 0, 1),
        TVec3::new(1, 0, 1),
    ];

    pub const FACES: [[TVec3<u8>; 6]; 6] = [FRONT, BACK, LEFT, RIGHT, UP, DOWN];
    pub const ADDENDS: [(i8, i8, i8); 6] = [
        (-1, 0, 0),
        (1, 0, 0),
        (0, 0, -1),
        (0, 0, 1),
        (0, 1, 0),
        (0, -1, 0),
    ];
    pub const LIGHT_MODIFIERS: [u32; 6] = [1, 1, 2, 2, 3, 0];
}

fn mesh(
    blocks: &[BlockId; BLOCKS_PER_CHUNK],
    neighbours: &[Option<Arc<Chunk>>; 6],
    buff: &mut [Vertex],
) -> usize {
    use meshing_consts::*;

    let mut i = 0;

    let mut emit_face = |pos: LocalBlockPos, face_idx: usize| {
        for local_pos in FACES[face_idx].iter() {
            let pos = *pos + local_pos;
            let data: u32 = pos.x as u32
                | (pos.y as u32) << 6
                | (pos.z as u32) << 12
                | LIGHT_MODIFIERS[face_idx] << 18;
            let vertex = Vertex { data };
            buff[i] = vertex;
            i += 1;
        }
    };

    let block_exists = |block_pos: LocalBlockPos, dir: usize| -> bool {
        let addend = ADDENDS[dir];
        let pos = block_pos.add(addend.0, addend.1, addend.2);
        if let Some(pos) = pos {
            blocks[pos.to_index()] != BlockId::Air
        } else {
            let neighbour = &neighbours[dir];
            if let Some(chunk) = neighbour {
                let blocks = chunk.blocks.read().expect("Lock poisoned");
                if let Some(ref blocks) = *blocks {
                    let block_pos = match dir {
                        0 => LocalBlockPos::new(CHUNK_SIZE as u8 - 1, block_pos.y, block_pos.z),
                        1 => LocalBlockPos::new(0, block_pos.y, block_pos.z),
                        2 => LocalBlockPos::new(block_pos.x, block_pos.y, CHUNK_SIZE as u8 - 1),
                        3 => LocalBlockPos::new(block_pos.x, block_pos.y, 0),
                        4 => LocalBlockPos::new(block_pos.x, 0, block_pos.z),
                        5 => LocalBlockPos::new(block_pos.x, CHUNK_SIZE as u8 - 1, block_pos.z),
                        _ => unreachable!(),
                    };
                    blocks[block_pos.to_index()] != BlockId::Air
                } else {
                    true
                }
            } else {
                false
            }
        }
    };

    for (i, &block) in blocks.iter().enumerate() {
        let pos = LocalBlockPos::from_index(i);

        if block != BlockId::Air {
            for dir in 0..6 {
                if !block_exists(pos, dir) {
                    emit_face(pos, dir);
                }
            }
        }
    }

    i
}

#[cfg(test)]
mod tests {
    use crate::world::MAX_VERTICES_PER_CHUNK;

    use super::*;
    use test::Bencher;

    #[bench]
    fn mesh(b: &mut Bencher) {
        let mut blocks = [BlockId::Air; BLOCKS_PER_CHUNK];
        for (i, block) in blocks.iter_mut().enumerate() {
            if i % 4 == 0 {
                *block = BlockId::Block;
            }
        }
        let mut buff = vec![Vertex { data: 0 }; MAX_VERTICES_PER_CHUNK];
        let neighbours = [None, None, None, None, None, None];
        b.iter(|| super::mesh(&blocks, &neighbours, &mut buff))
    }
}
