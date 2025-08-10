use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use ksync::{AsyncMutex, AsyncMutexGuard};

use super::IExtFs;
use crate::{
    fs::vfs::basic::superblock::{SuperBlock, SuperBlockMeta},
    include::fs::Statfs,
};

pub struct Ext4SuperBlock {
    meta: SuperBlockMeta,
    pub inner: Arc<AsyncMutex<IExtFs>>,
}

impl Ext4SuperBlock {
    pub fn new(meta: SuperBlockMeta, inner: Arc<AsyncMutex<IExtFs>>) -> Self {
        Self { meta, inner }
    }
    pub async fn get_fs(&self) -> AsyncMutexGuard<IExtFs> {
        self.inner.lock().await
    }
}

#[async_trait]
impl SuperBlock for Ext4SuperBlock {
    fn meta(&self) -> &SuperBlockMeta {
        &self.meta
    }
    async fn statfs(&self) -> include::errno::SysResult<crate::include::fs::Statfs> {
        let superblock = self.get_fs().await.super_block;
        Ok(Statfs {
            f_type: 0xEF53,
            f_bsize: superblock.block_size() as u64,
            f_blocks: superblock.blocks_count() as u64,
            f_bfree: superblock.free_blocks_count() as u64,
            f_bavail: superblock.free_blocks_count() / 10 * 9 as u64, /* ext4文件系统默认保留了整个磁盘空间的5%作为预留空间 */
            f_files: superblock.total_inodes() as u64,
            f_ffree: superblock.free_inodes_count() as u64,
            f_fsid: 0, // not use
            f_namelen: 255,
            f_frsize: superblock.block_size() as u64,
            f_flag: 0,       // not use
            f_spare: [0; 4], // not use
        })
    }
}
