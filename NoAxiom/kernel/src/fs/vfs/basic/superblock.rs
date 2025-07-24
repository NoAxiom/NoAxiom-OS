use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};

use async_trait::async_trait;
use downcast_rs::{impl_downcast, DowncastSync};
use driver::devices::block::BlockDevice;
use ksync::Once;

use super::{
    dentry::Dentry,
    filesystem::{EmptyFileSystem, FileSystem},
};

/// stand for file system
pub struct SuperBlockMeta {
    /// The device of the file system, None if it is a virtual file system
    device: Option<&'static dyn BlockDevice>,
    /// The file system
    file_system: Arc<dyn FileSystem>,
    /// The root of the file system, use weak to avoid reference cycle
    root: Once<Weak<dyn Dentry>>,
}

impl SuperBlockMeta {
    pub fn new(device: Option<&'static dyn BlockDevice>, file_system: Arc<dyn FileSystem>) -> Self {
        Self {
            device,
            file_system,
            root: Once::new(),
        }
    }
}

#[async_trait]
pub trait SuperBlock: Send + Sync + DowncastSync {
    fn meta(&self) -> &SuperBlockMeta;
    async fn sync_all(&self) {
        if let Some(dev) = &self.meta().device {
            dev.sync_all().await.expect("sync all failed");
        }
    }
}
impl_downcast!(sync SuperBlock);

pub struct EmptySuperBlock {
    meta: SuperBlockMeta,
}

impl EmptySuperBlock {
    pub fn new() -> Self {
        Self {
            meta: SuperBlockMeta {
                device: None,
                file_system: Arc::new(EmptyFileSystem::new()),
                root: Once::new(),
            },
        }
    }
}

impl SuperBlock for EmptySuperBlock {
    fn meta(&self) -> &SuperBlockMeta {
        &self.meta
    }
}
