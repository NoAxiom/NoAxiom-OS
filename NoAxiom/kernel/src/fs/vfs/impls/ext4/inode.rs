use alloc::{boxed::Box, sync::Arc};

use arch::{Arch, ArchInt};
use async_trait::async_trait;
use include::errno::Errno;
use ksync::{mutex::SpinLock, AsyncMutex};

use super::{fs_err, superblock::Ext4SuperBlock, IExtInode};
use crate::{
    config::fs::BLOCK_SIZE,
    fs::vfs::basic::{
        inode::{Inode, InodeMeta},
        superblock::SuperBlock,
    },
    include::fs::{InodeMode, Stat},
    syscall::SysResult,
};

pub struct Ext4FileInode {
    meta: InodeMeta,
    ino: Arc<AsyncMutex<IExtInode>>,
}

impl Ext4FileInode {
    pub fn new(superblock: Arc<dyn SuperBlock>, inode: IExtInode, mode: InodeMode) -> Self {
        let file_size = inode.inode.size();
        Self {
            meta: InodeMeta::new(superblock, InodeMode::FILE | mode, file_size as usize, true),
            ino: Arc::new(AsyncMutex::new(inode)),
        }
    }
    pub fn get_inode(&self) -> Arc<AsyncMutex<IExtInode>> {
        self.ino.clone()
    }
}

#[async_trait]
impl Inode for Ext4FileInode {
    #[inline(always)]
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::include::fs::Stat, crate::include::result::Errno> {
        let inner = self.meta.inner.lock();
        let mode = self
            .meta
            .inode_mode
            .load(core::sync::atomic::Ordering::SeqCst);
        debug!("[Ext4FileInode] mode: {:#o}", mode);
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
        let super_block = &self.meta.super_block;
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs()
            .await;
        let mut inode = self.ino.lock().await;
        debug!(
            "[Ext4FileInode] truncate inode: {}, new_size: {}",
            inode.inode_num, new
        );
        assert_no_lock!();
        assert!(Arch::is_interrupt_enabled());

        let old_size = inode.inode.size() as usize;
        if new <= old_size {
            ext4.truncate_inode(&mut inode, new as u64)
                .await
                .map_err(fs_err)?;
        } else {
            ext4.write_at(inode.inode_num, new, &[0; 1])
                .await
                .map_err(fs_err)?;
        }
        Ok(())
    }
}

pub struct Ext4DirInode {
    meta: InodeMeta,
    ino: Arc<SpinLock<IExtInode>>,
}

impl Ext4DirInode {
    pub fn new(superblock: Arc<dyn SuperBlock>, inode: IExtInode, mode: InodeMode) -> Self {
        Self {
            meta: InodeMeta::new(superblock, InodeMode::DIR | mode, 0, false),
            ino: Arc::new(SpinLock::new(inode)),
        }
    }
    pub fn get_inode(&self) -> Arc<SpinLock<IExtInode>> {
        self.ino.clone()
    }
}

#[async_trait]
impl Inode for Ext4DirInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::include::fs::Stat, crate::include::result::Errno> {
        let inner = self.meta.inner.lock();
        let mode = self
            .meta
            .inode_mode
            .load(core::sync::atomic::Ordering::SeqCst);
        debug!("[Ext4FileInode] mode: {:#o}", mode);
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
    async fn truncate(&self, _new: usize) -> SysResult<()> {
        Err(Errno::EINVAL)
    }
}
