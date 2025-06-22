use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};

use async_trait::async_trait;
use driver::devices::impls::device::BlockDevice;

use super::{dentry::Dentry, superblock::SuperBlock};
use crate::include::fs::MountFlags;

pub struct FileSystemMeta {
    pub name: String,
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
        device: Option<Arc<&'static dyn BlockDevice>>,
    ) -> Arc<dyn Dentry>;
}

pub struct EmptyFileSystem {
    meta: FileSystemMeta,
}

impl EmptyFileSystem {
    pub fn new() -> Self {
        Self {
            meta: FileSystemMeta::new("EmptyFS"),
        }
    }
}

#[async_trait]
impl FileSystem for EmptyFileSystem {
    fn meta(&self) -> &FileSystemMeta {
        &self.meta
    }

    async fn root(
        self: Arc<Self>,
        _parent: Option<Arc<dyn Dentry>>,
        _flags: MountFlags,
        _name: &str,
        _device: Option<Arc<&'static dyn BlockDevice>>,
    ) -> Arc<dyn Dentry> {
        unreachable!()
    }
}
