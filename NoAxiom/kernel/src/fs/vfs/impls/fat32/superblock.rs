use alloc::sync::Arc;

use crate::fs::{
    fat32::FAT32FIleSystem,
    vfs::basic::superblock::{SuperBlock, SuperBlockMeta},
};

pub struct FAT32SuperBlock {
    meta: SuperBlockMeta,
    pub inner: Arc<FAT32FIleSystem>,
}

impl FAT32SuperBlock {
    pub fn new(meta: SuperBlockMeta, inner: Arc<FAT32FIleSystem>) -> Self {
        Self { meta, inner }
    }
}

impl SuperBlock for FAT32SuperBlock {
    fn meta(&self) -> &SuperBlockMeta {
        &self.meta
    }
}
