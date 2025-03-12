use alloc::sync::Arc;

use spin::{Mutex, MutexGuard};

use super::IExtFs;
use crate::fs::vfs::basic::superblock::{SuperBlock, SuperBlockMeta};

pub struct Ext4SuperBlock {
    meta: SuperBlockMeta,
    pub inner: Arc<Mutex<IExtFs>>,
}

impl Ext4SuperBlock {
    pub fn new(meta: SuperBlockMeta, inner: Arc<Mutex<IExtFs>>) -> Self {
        Self { meta, inner }
    }
    pub fn get_fs(&self) -> MutexGuard<IExtFs> {
        self.inner.lock()
    }
}

impl SuperBlock for Ext4SuperBlock {
    fn meta(&self) -> &SuperBlockMeta {
        &self.meta
    }
}
