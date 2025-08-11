use alloc::{boxed::Box, sync::Arc};

use crate::{
    config::fs::BLOCK_SIZE,
    fs::vfs::basic::{
        inode::{Inode, InodeMeta},
        superblock::SuperBlock,
    },
    include::fs::{InodeMode, Stat},
    syscall::SysResult,
};
pub struct RamFsFileInode {
    meta: InodeMeta,
}

impl RamFsFileInode {
    pub fn new(superblock: Arc<dyn SuperBlock>, size: usize) -> Self {
        Self {
            meta: InodeMeta::new(
                superblock,
                InodeMode::FILE | InodeMode::from_bits(0o444).unwrap(),
                size,
                false,
            ),
        }
    }
}

#[async_trait::async_trait]
impl Inode for RamFsFileInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::include::fs::Stat, crate::include::result::Errno> {
        let inner = self.meta.inner.lock();
        let mode = self
            .meta
            .inode_mode
            .load(core::sync::atomic::Ordering::SeqCst);
        Ok(Stat {
            st_dev: 0,
            st_ino: self.meta.id as u64,
            st_mode: mode,
            st_nlink: 1,
            st_uid: self.meta.uid.load(core::sync::atomic::Ordering::SeqCst),
            st_gid: self.meta.gid.load(core::sync::atomic::Ordering::SeqCst),
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
    async fn truncate(&self, new: usize) -> SysResult<()> {
        self.meta.inner.lock().size = new;
        Ok(())
    }
}

pub struct RamFsDirInode {
    meta: InodeMeta,
}

impl RamFsDirInode {
    pub fn new(superblock: Arc<dyn SuperBlock>, size: usize) -> Self {
        Self {
            meta: InodeMeta::new(
                superblock,
                InodeMode::DIR | InodeMode::from_bits(0o755).unwrap(),
                size,
                false,
            ),
        }
    }
}

impl Inode for RamFsDirInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::include::fs::Stat, crate::include::result::Errno> {
        let inner = self.meta.inner.lock();
        let mode = self
            .meta
            .inode_mode
            .load(core::sync::atomic::Ordering::SeqCst);
        Ok(Stat {
            st_dev: 0,
            st_ino: self.meta.id as u64,
            st_mode: mode,
            st_nlink: 1,
            st_uid: self.meta.uid.load(core::sync::atomic::Ordering::SeqCst),
            st_gid: self.meta.gid.load(core::sync::atomic::Ordering::SeqCst),
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
