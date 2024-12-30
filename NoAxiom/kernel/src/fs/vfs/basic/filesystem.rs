use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};

use async_trait::async_trait;

use super::{dentry::Dentry, superblock::SuperBlock};
use crate::{device::block::BlockDevice, nix::fs::MountFlags};

pub struct FileSystemMeta {
    name: String,
    super_block: Option<Arc<dyn SuperBlock>>,
}

impl FileSystemMeta {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            super_block: None,
        }
    }
}

#[async_trait]
pub trait FileSystem: Send + Sync {
    fn meta(&self) -> &FileSystemMeta;
    async fn root(
        self: Arc<Self>,
        parent: Option<Arc<dyn Dentry>>,
        flags: MountFlags,
        name: &str,
        device: Option<Arc<dyn BlockDevice>>,
    ) -> Arc<dyn Dentry>;
}
