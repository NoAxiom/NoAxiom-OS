use alloc::sync::Arc;

use crate::{
    config::fs::BLOCK_SIZE,
    fs::vfs::basic::{
        inode::{Inode, InodeMeta},
        superblock::SuperBlock,
    },
    include::fs::{InodeMode, MemInfo, Stat},
};

pub struct StatusInode {
    meta: InodeMeta,
}

impl StatusInode {
    pub fn new(superblock: Arc<dyn SuperBlock>) -> Self {
        let meminfo = Arc::new(MemInfo::new());
        let file_size = meminfo.serialize().len();
        Self {
            meta: InodeMeta::new(superblock, InodeMode::FILE, file_size, false),
        }
    }
}

impl Inode for StatusInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::include::fs::Stat, crate::include::result::Errno> {
        let inner = self.meta.inner.lock();
        let mode = self.meta.inode_mode.bits();
        Ok(Stat {
            st_dev: 0,
            st_ino: self.meta.id as u64,
            st_mode: mode,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
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
