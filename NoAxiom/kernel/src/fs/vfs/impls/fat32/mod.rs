use alloc::sync::Arc;

use crate::fs::vfs::basic::dentry::Dentry;

pub mod dentry;
pub mod file;
pub mod filesystem;
pub mod inode;
pub mod superblock;

// todo: check all the CLONE expenses

pub fn fat_init() -> Arc<dyn Dentry> {
    todo!()
}
