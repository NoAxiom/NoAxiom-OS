use alloc::sync::Arc;

use ksync::async_mutex::{AsyncMutex, AsyncMutexGuard};

use super::IExtFs;
use crate::fs::vfs::basic::superblock::{SuperBlock, SuperBlockMeta};

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

impl SuperBlock for Ext4SuperBlock {
    fn meta(&self) -> &SuperBlockMeta {
        &self.meta
    }
}
