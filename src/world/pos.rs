use std::{
    fmt::{Debug, Display},
    mem::size_of,
    ops::{Add, Deref, DerefMut},
    slice,
};

use nalgebra_glm::{TVec2, TVec3, Vec3};

use crate::world::CHUNK_SIZE;

use super::REGION_SIZE;

/// The position of a block in a chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct LocalBlockPos {
    x: u8,
    y: u8,
    z: u8,
}

impl LocalBlockPos {
    #[inline(always)]
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        debug_assert!(x < CHUNK_SIZE as u8, "x too big: {x}");
        debug_assert!(y < CHUNK_SIZE as u8, "y too big: {y}");
        debug_assert!(z < CHUNK_SIZE as u8, "z too big: {z}");
        Self { x, y, z }
    }

    #[inline(always)]
    pub fn try_new(x: i8, y: i8, z: i8) -> Option<Self> {
        if x >= CHUNK_SIZE as _ || y >= CHUNK_SIZE as _ || z >= CHUNK_SIZE as _ {
            return None;
        }
        Some(Self::new(
            x.try_into().ok()?,
            y.try_into().ok()?,
            z.try_into().ok()?,
        ))
    }

    #[inline(always)]
    pub fn to_index(self) -> usize {
        (self.x as usize * CHUNK_SIZE + self.y as usize) * CHUNK_SIZE + self.z as usize
    }
}

/// The position of a chunk in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub struct ChunkPos {
    x: i64,
    y: i64,
    z: i64,
}

impl ChunkPos {
    #[inline(always)]
    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) }
    }

    #[inline(always)]
    pub fn xyz(&self) -> (i64, i64, i64) {
        (self.x, self.y, self.z)
    }

    #[inline(always)]
    pub fn flat(&self) -> FlatChunkPos {
        FlatChunkPos::new(self.x, self.z)
    }

    #[inline(always)]
    pub fn x(&self) -> i64 {
        self.x
    }
    #[inline(always)]
    pub fn y(&self) -> i64 {
        self.y
    }
    #[inline(always)]
    pub fn z(&self) -> i64 {
        self.z
    }

    #[inline(always)]
    pub fn region(&self) -> RegionPos {
        let mut x = self.x / REGION_SIZE as i64;
        let mut y = self.y / REGION_SIZE as i64;
        let mut z = self.z / REGION_SIZE as i64;
        if self.x < 0 && self.x % REGION_SIZE as i64 != 0 {
            x -= 1;
        }
        if self.y < 0 && self.y % REGION_SIZE as i64 != 0 {
            y -= 1;
        }
        if self.z < 0 && self.z % REGION_SIZE as i64 != 0 {
            z -= 1;
        }
        RegionPos::new(x, y, z)
    }

    #[inline(always)]
    pub fn between(&self, a: &Self, b: &Self) -> bool {
        self.x >= a.x
            && self.y >= a.y
            && self.z >= a.z
            && self.x < b.x
            && self.y < b.y
            && self.z < b.z
    }
}
impl Add for ChunkPos {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
impl Display for ChunkPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.x, self.y, self.z)
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
        let (x, y, z) = self.chunk_pos.xyz();
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
    pub const fn new(x: f32, y: f32, z: f32, pitch: f32, yaw: f32) -> Self {
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
        let mut x = (self.pos.x / CHUNK_SIZE as f32) as i64;
        let mut y = (self.pos.y / CHUNK_SIZE as f32) as i64;
        let mut z = (self.pos.z / CHUNK_SIZE as f32) as i64;
        if self.pos.x < 0. {
            x -= 1;
        }
        if self.pos.y < 0. {
            y -= 1;
        }
        if self.pos.z < 0. {
            z -= 1;
        }
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
impl Display for EntityPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.2} {:.2} {:.2} {:.2} {:.2}",
            self.pos.x,
            self.pos.y,
            self.pos.z,
            self.pitch(),
            self.yaw()
        )
    }
}

// The 2D position of a chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlatChunkPos {
    inner: TVec2<i64>,
}

impl FlatChunkPos {
    #[inline(always)]
    pub fn new(x: i64, z: i64) -> Self {
        Self {
            inner: TVec2::new(x, z),
        }
    }
    #[inline(always)]
    pub fn x(&self) -> i64 {
        self.inner.x
    }
    #[inline(always)]
    pub fn z(&self) -> i64 {
        self.inner.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionPos {
    x: i64,
    y: i64,
    z: i64,
}

impl RegionPos {
    #[inline(always)]
    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    #[inline(always)]
    pub fn x(&self) -> i64 {
        self.x
    }
    #[inline(always)]
    pub fn y(&self) -> i64 {
        self.y
    }
    #[inline(always)]
    pub fn z(&self) -> i64 {
        self.z
    }
}
impl Add for RegionPos {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}
impl Display for RegionPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.x, self.y, self.z)
    }
}
