#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockId {
    Air = 0,
    Block,
}
