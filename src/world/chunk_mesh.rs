use std::{mem, sync::Arc};

use crate::{
    render::Vertex,
    world::{LocalBlockPos, CHUNK_SIZE, MAX_VERTICES_PER_CHUNK},
};

use super::{blocks::BlockId, chunk::Chunk, BLOCKS_PER_CHUNK};

pub const ADDENDS: [(i8, i8, i8); 6] = [
    (1, 0, 0),
    (-1, 0, 0),
    (0, 1, 0),
    (0, -1, 0),
    (0, 0, 1),
    (0, 0, -1),
];
pub const LIGHT_MODIFIERS: [u32; 6] = [1, 1, 3, 0, 2, 2];

#[inline(always)]
fn block_exist(
    blocks: &[BlockId; BLOCKS_PER_CHUNK],
    neighbours: &[Option<Arc<Chunk>>; 6],
    block_pos: [i8; 3],
    addend: [i8; 3],
) -> bool {
    let pos = [
        block_pos[0] + addend[0],
        block_pos[1] + addend[1],
        block_pos[2] + addend[2],
    ];

    let local_pos = LocalBlockPos::try_new(pos[0], pos[1], pos[2]);
    if let Some(pos) = local_pos {
        blocks[pos.to_index()] != BlockId::Air
    } else {
        let (neighbour, pos) = if pos[0] >= CHUNK_SIZE as _ {
            (
                0,
                LocalBlockPos::new(0, block_pos[1] as _, block_pos[2] as _),
            )
        } else if pos[0] < 0 {
            (
                1,
                LocalBlockPos::new(CHUNK_SIZE as u8 - 1, block_pos[1] as _, block_pos[2] as _),
            )
        } else if pos[1] >= CHUNK_SIZE as _ {
            (
                2,
                LocalBlockPos::new(block_pos[0] as _, 0, block_pos[2] as _),
            )
        } else if pos[1] < 0 {
            (
                3,
                LocalBlockPos::new(block_pos[0] as _, CHUNK_SIZE as u8 - 1, block_pos[2] as _),
            )
        } else if pos[2] >= CHUNK_SIZE as _ {
            (
                4,
                LocalBlockPos::new(block_pos[0] as _, block_pos[1] as _, 0),
            )
        } else if pos[2] < 0 {
            (
                5,
                LocalBlockPos::new(block_pos[0] as _, block_pos[1] as _, CHUNK_SIZE as u8 - 1),
            )
        } else {
            unreachable!()
        };

        let neighbour = &neighbours[neighbour];
        if let Some(chunk) = neighbour {
            let blocks = chunk.blocks.read().expect("Lock poisoned");
            blocks.data[pos.to_index()] != BlockId::Air
        } else {
            false
        }
    }
}

#[inline(always)]
fn build_vert(pos: (u8, u8, u8), light_modifier: u32) -> Vertex {
    let data = pos.0 as u32 | (pos.1 as u32) << 6 | (pos.2 as u32) << 12 | light_modifier << 18;
    Vertex { data }
}

#[inline(always)]
fn append_quad(buff: &mut [Vertex], buff_idx: &mut usize, points: [(i8, i8, i8); 4], dir: usize) {
    debug_assert!(points.iter().all(|&p| p >= (0, 0, 0)));
    let points: [(u8, u8, u8); 4] = unsafe { mem::transmute(points) };
    let light_modifier = LIGHT_MODIFIERS[dir];
    let verts: [Vertex; 4] = [
        build_vert(points[0], light_modifier),
        build_vert(points[1], light_modifier),
        build_vert(points[2], light_modifier),
        build_vert(points[3], light_modifier),
    ];

    let idx = *buff_idx;

    // select vertex order for culling
    if dir % 2 == 0 {
        buff[idx] = verts[0];
        buff[idx + 1] = verts[2];
        buff[idx + 2] = verts[1];
        buff[idx + 3] = verts[1];
        buff[idx + 4] = verts[2];
        buff[idx + 5] = verts[3];
    } else {
        buff[idx] = verts[0];
        buff[idx + 1] = verts[1];
        buff[idx + 2] = verts[2];
        buff[idx + 3] = verts[1];
        buff[idx + 4] = verts[3];
        buff[idx + 5] = verts[2];
    }
    *buff_idx += 6;
}

#[inline]
pub fn mesh(
    blocks: &[BlockId; BLOCKS_PER_CHUNK],
    neighbours: &[Option<Arc<Chunk>>; 6],
    buff: &mut [Vertex],
) -> usize {
    // here to gain ~5us/iter
    assert!(buff.len() == MAX_VERTICES_PER_CHUNK);
    let mut buff_idx = 0;
    for d in 0..3 {
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;

        let mut x = [0; 3];
        let mut q = [0; 3];

        let mut mask = [0_u8; CHUNK_SIZE * CHUNK_SIZE];

        q[d] = 1;
        x[d] = -1;
        while x[d] < CHUNK_SIZE as i8 {
            let mut n = 0;
            x[v] = 0;
            while x[v] < CHUNK_SIZE as i8 {
                x[u] = 0;
                while x[u] < CHUNK_SIZE as i8 {
                    let block_current_exists = block_exist(blocks, neighbours, x, [0, 0, 0]);
                    let block_compare_exists = block_exist(blocks, neighbours, x, q);
                    mask[n] = match (block_current_exists, block_compare_exists) {
                        (true, true) => 0,
                        (false, false) => 0,
                        (true, false) => 1,
                        (false, true) => 2,
                    };
                    n += 1;
                    x[u] += 1;
                }
                x[v] += 1;
            }

            x[d] += 1;
            n = 0;

            for j in 0..CHUNK_SIZE {
                let mut i = 0;
                while i < CHUNK_SIZE {
                    if mask[n] != 0 {
                        let mut w = 1;
                        let mut last_mask = mask[n];
                        while i + w < CHUNK_SIZE && mask[n + w] != 0 && mask[n + w] == last_mask {
                            last_mask = mask[n + w];
                            w += 1;
                        }

                        let mut h = 1;
                        last_mask = mask[n];
                        'a: while j + h < CHUNK_SIZE {
                            for k in 0..w {
                                let m = mask[n + k + h * CHUNK_SIZE];
                                if m == 0 || m != last_mask {
                                    break 'a;
                                }
                                last_mask = m;
                            }

                            h += 1;
                        }

                        x[u] = i as _;
                        x[v] = j as _;

                        let mut du = [0; 3];
                        du[u] = w as _;

                        let mut dv = [0; 3];
                        dv[v] = h as _;

                        append_quad(
                            buff,
                            &mut buff_idx,
                            [
                                (x[0], x[1], x[2]),
                                (x[0] + du[0], x[1] + du[1], x[2] + du[2]),
                                (x[0] + dv[0], x[1] + dv[1], x[2] + dv[2]),
                                (
                                    x[0] + dv[0] + du[0],
                                    x[1] + dv[1] + du[1],
                                    x[2] + dv[2] + du[2],
                                ),
                            ],
                            d * 2 + mask[n] as usize - 1,
                        );

                        for l in 0..h {
                            for k in 0..w {
                                mask[n + k + l * CHUNK_SIZE] = 0;
                            }
                        }

                        i += w;
                        n += w;
                    } else {
                        i += 1;
                        n += 1;
                    }
                }
            }
        }
    }

    buff_idx
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

        b.iter(|| {
            super::mesh(&blocks, &neighbours, &mut buff);
        })
    }
}
