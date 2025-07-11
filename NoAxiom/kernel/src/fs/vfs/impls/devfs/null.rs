use alloc::sync::Arc;

use crate::{
    config::fs::BLOCK_SIZE,
    dentry_default, file_default,
    fs::vfs::basic::{
        inode::{Inode, InodeMeta},
        superblock::SuperBlock,
    },
    include::fs::{InodeMode, Stat},
};

file_default!(
    NullFile,
    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        Ok(0)
    },
    async fn base_write(&self, _offset: usize, buf: &[u8]) -> SyscallResult {
        Ok(buf.len() as isize)
    }
);

dentry_default!(NullDentry, NullFile);

pub struct NullInode {
    meta: InodeMeta,
}

impl NullInode {
    pub fn new(superblock: Arc<dyn SuperBlock>) -> Self {
        Self {
            meta: InodeMeta::new(superblock, InodeMode::CHAR, BLOCK_SIZE, false),
        }
    }
}

impl Inode for NullInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::include::fs::Stat, crate::include::result::Errno> {
        let inner = self.meta.inner.lock();
        let mode = self.meta.inode_mode.bits();
        Ok(Stat {
            st_dev: 1,
            st_ino: self.meta.id as u64,
            st_mode: mode,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: (1 << 8) | 0x3,
            __pad: 0,
            st_size: inner.size as u64,
            st_blksize: BLOCK_SIZE as u32,
            __pad2: 0,
            st_blocks: (inner.size / 512) as u64,
            st_atime_sec: inner.atime_sec as u64,
            st_atime_nsec: inner.atime_nsec as u64,
            st_mtime_sec: inner.mtime_sec as u64,
            st_mtime_nsec: inner.mtime_nsec as u64,
            st_ctime_sec: inner.ctime_sec as u64,
            st_ctime_nsec: inner.ctime_nsec as u64,
            unused: 0,
        })
    }
}
