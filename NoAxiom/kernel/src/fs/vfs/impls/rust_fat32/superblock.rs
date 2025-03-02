use alloc::sync::Arc;

use super::IFatFs;
use crate::fs::vfs::basic::superblock::{SuperBlock, SuperBlockMeta};

pub struct Fat32SuperBlock {
    meta: SuperBlockMeta,
    pub inner: Arc<IFatFs>,
}

impl Fat32SuperBlock {
    pub fn new(meta: SuperBlockMeta, inner: Arc<IFatFs>) -> Self {
        Self { meta, inner }
    }
}

impl SuperBlock for Fat32SuperBlock {
    fn meta(&self) -> &SuperBlockMeta {
        &self.meta
    }
}
