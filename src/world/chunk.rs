use log::trace;
use nalgebra_glm::TVec3;

use crate::{
    render::{Buffer, Vertex},
    world::LocalBlockPos,
};

use super::{blocks::BlockId, pos::ChunkPos, BLOCKS_PER_CHUNK, CHUNK_SIZE};

#[derive(Debug)]
pub struct Chunk {
    pub pos: ChunkPos,
    blocks: [BlockId; BLOCKS_PER_CHUNK],
    pub vertex_buffer: Option<Buffer>,
}

impl Chunk {
    pub fn generate(pos: ChunkPos) -> Self {
        trace!("Generate chunk {:?}", pos);
        let mut blocks = [BlockId::Air; BLOCKS_PER_CHUNK];
        for (i, block) in blocks.iter_mut().enumerate() {
            if i % 4 == 0 {
                *block = BlockId::Block;
            }
        }
        Self {
            pos,
            blocks,
            vertex_buffer: None,
        }
    }

    /// Return the count of vertices generated.
    pub fn mesh(&self, buff: &mut [Vertex]) -> usize {
        trace!("Mesh chunk {:?}", self.pos);
        const FRONT: [TVec3<u8>; 6] = [
            TVec3::new(0, 0, 0),
            TVec3::new(0, 1, 0),
            TVec3::new(0, 0, 1),
            TVec3::new(0, 1, 0),
            TVec3::new(0, 1, 1),
            TVec3::new(0, 0, 1),
        ];
        const BACK: [TVec3<u8>; 6] = [
            TVec3::new(1, 0, 0),
            TVec3::new(1, 0, 1),
            TVec3::new(1, 1, 0),
            TVec3::new(1, 1, 0),
            TVec3::new(1, 0, 1),
            TVec3::new(1, 1, 1),
        ];
        const LEFT: [TVec3<u8>; 6] = [
            TVec3::new(1, 0, 0),
            TVec3::new(1, 1, 0),
            TVec3::new(0, 0, 0),
            TVec3::new(1, 1, 0),
            TVec3::new(0, 1, 0),
            TVec3::new(0, 0, 0),
        ];
        const RIGHT: [TVec3<u8>; 6] = [
            TVec3::new(0, 0, 1),
            TVec3::new(0, 1, 1),
            TVec3::new(1, 0, 1),
            TVec3::new(0, 1, 1),
            TVec3::new(1, 1, 1),
            TVec3::new(1, 0, 1),
        ];
        const UP: [TVec3<u8>; 6] = [
            TVec3::new(0, 1, 0),
            TVec3::new(1, 1, 0),
            TVec3::new(0, 1, 1),
            TVec3::new(1, 1, 0),
            TVec3::new(1, 1, 1),
            TVec3::new(0, 1, 1),
        ];
        const DOWN: [TVec3<u8>; 6] = [
            TVec3::new(1, 0, 0),
            TVec3::new(0, 0, 0),
            TVec3::new(1, 0, 1),
            TVec3::new(0, 0, 0),
            TVec3::new(0, 0, 1),
            TVec3::new(1, 0, 1),
        ];

        let mut i = 0;

        let mut emit_face = |face: &[TVec3<u8>; 6], light_modifier: u8, pos: LocalBlockPos| {
            for local_pos in face.iter() {
                let vertex = Vertex {
                    pos: *pos + local_pos,
                    light_modifier,
                };
                buff[i] = vertex;
                i += 1;
            }
        };

        for i in 0..BLOCKS_PER_CHUNK {
            let pos = LocalBlockPos::from_index(i);
            let block = self.blocks[i];
            if block != BlockId::Air {
                if pos.x as usize >= CHUNK_SIZE - 1
                    || self.blocks[pos.add(1, 0, 0).to_index()] == BlockId::Air
                {
                    emit_face(&BACK, 6, pos);
                }
                if pos.x == 0 || self.blocks[pos.add(-1, 0, 0).to_index()] == BlockId::Air {
                    emit_face(&FRONT, 6, pos);
                }
                if pos.z as usize >= CHUNK_SIZE - 1
                    || self.blocks[pos.add(0, 0, 1).to_index()] == BlockId::Air
                {
                    emit_face(&RIGHT, 8, pos);
                }
                if pos.z == 0 || self.blocks[pos.add(0, 0, -1).to_index()] == BlockId::Air {
                    emit_face(&LEFT, 8, pos);
                }
                if pos.y as usize >= CHUNK_SIZE - 1
                    || self.blocks[pos.add(0, 1, 0).to_index()] == BlockId::Air
                {
                    emit_face(&UP, 10, pos);
                }
                if pos.y == 0 || self.blocks[pos.add(0, -1, 0).to_index()] == BlockId::Air {
                    emit_face(&DOWN, 4, pos);
                }
            }
        }

        i
    }
}
