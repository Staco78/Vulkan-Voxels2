use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use nalgebra_glm::{TVec2, TVec3, Vec3};

use crate::world::{BLOCKS_PER_CHUNK, CHUNK_SIZE};

/// The position of a block in a chunk.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct LocalBlockPos {
    inner: TVec3<u8>,
}

impl LocalBlockPos {
    #[inline(always)]
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        debug_assert!(x < CHUNK_SIZE as u8, "x too big: {x}");
        debug_assert!(y < CHUNK_SIZE as u8, "y too big: {y}");
        debug_assert!(z < CHUNK_SIZE as u8, "z too big: {z}");
        Self {
            inner: TVec3::new(x, y, z),
        }
    }

    #[inline(always)]
    pub fn from_index(index: usize) -> Self {
        debug_assert!(index < BLOCKS_PER_CHUNK);
        let x = index / (CHUNK_SIZE * CHUNK_SIZE);
        let y = (index % (CHUNK_SIZE * CHUNK_SIZE)) / CHUNK_SIZE;
        let z = index % CHUNK_SIZE;
        Self::new(x as u8, y as u8, z as u8)
    }

    #[inline(always)]
    pub fn to_index(self) -> usize {
        let &[x, y, z] = self.inner.as_slice() else {
            unreachable!()
        };
        (x as usize * CHUNK_SIZE + y as usize) * CHUNK_SIZE + z as usize
    }

    /// # Panic
    /// Panic if the new coordinate is outside of 0..CHUNK_SIZE.
    #[inline(always)]
    pub fn add(self, x: i8, y: i8, z: i8) -> Self {
        let nx = self.x.checked_add_signed(x).expect("self.x - x < 0");
        let ny = self.y.checked_add_signed(y).expect("self.y - y < 0");
        let nz = self.z.checked_add_signed(z).expect("self.z - z < 0");
        assert!((nx as usize) < CHUNK_SIZE);
        assert!((ny as usize) < CHUNK_SIZE);
        assert!((nz as usize) < CHUNK_SIZE);
        Self::new(nx, ny, nz)
    }
}

impl Deref for LocalBlockPos {
    type Target = TVec3<u8>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl Debug for LocalBlockPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// The position of a chunk in the world.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct ChunkPos {
    inner: TVec3<i64>,
}

impl ChunkPos {
    #[inline(always)]
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        Self {
            inner: TVec3::new(x, y, z),
        }
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        let (a, bytes, b) = unsafe { self.inner.as_slice().align_to::<u8>() };
        debug_assert!(a.is_empty());
        debug_assert!(b.is_empty());
        bytes
    }

    #[inline(always)]
    pub fn xyz(&self) -> (i64, i64, i64) {
        let &[x, y, z] = self.inner.as_slice() else {
            unreachable!()
        };
        (x, y, z)
    }
}

impl Deref for ChunkPos {
    type Target = TVec3<i64>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl Debug for ChunkPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// The position on a block in the world.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct BlockPos {
    chunk_pos: ChunkPos,
    local_pos: LocalBlockPos,
}

impl BlockPos {
    pub fn to_vec(self) -> TVec3<i128> {
        let &[x, y, z] = self.chunk_pos.as_slice() else {
            unreachable!()
        };
        let (x, y, z) = (
            x as i128 * CHUNK_SIZE as i128,
            y as i128 * CHUNK_SIZE as i128,
            z as i128 * CHUNK_SIZE as i128,
        );
        let l = self.local_pos;
        let (x, y, z) = (x + l.x as i128, y + l.y as i128, z + l.z as i128);
        TVec3::new(x, y, z)
    }
}

impl Debug for BlockPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pos = self.to_vec();
        write!(f, "({}, {}, {})", pos.x, pos.y, pos.z)
    }
}

/// The position and the look direction of an entity.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct EntityPos {
    pub pos: Vec3,
    pub look: TVec2<f32>,
}

impl EntityPos {
    #[inline(always)]
    pub fn new(x: f32, y: f32, z: f32, pitch: f32, yaw: f32) -> Self {
        Self {
            pos: TVec3::new(x, y, z),
            look: TVec2::new(pitch, yaw),
        }
    }
    #[inline(always)]
    pub fn pitch(&self) -> f32 {
        self.look.x
    }
    #[inline(always)]
    pub fn yaw(&self) -> f32 {
        self.look.y
    }
    #[inline(always)]
    pub fn chunk(&self) -> ChunkPos {
        let x = (self.pos.x / CHUNK_SIZE as f32) as i64;
        let y = (self.pos.y / CHUNK_SIZE as f32) as i64;
        let z = (self.pos.z / CHUNK_SIZE as f32) as i64;
        ChunkPos::new(x, y, z)
    }
}
impl Deref for EntityPos {
    type Target = Vec3;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.pos
    }
}
impl DerefMut for EntityPos {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pos
    }
}
impl Debug for EntityPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {}, {}, {})",
            self.pos.x,
            self.pos.y,
            self.pos.z,
            self.pitch(),
            self.yaw()
        )
    }
}
