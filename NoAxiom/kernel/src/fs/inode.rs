//! for os use: Inode
//! provide the interface for file system

pub struct Inode {
    pub id: usize,
    pub size: usize,
    pub block_id: usize,
    pub block_size: usize,
}
