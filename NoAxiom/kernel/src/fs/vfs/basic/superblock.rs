use alloc::sync::{Arc, Weak};

use downcast_rs::{impl_downcast, DowncastSync};
use ksync::Once;

use super::{
    dentry::Dentry,
    filesystem::{EmptyFileSystem, FileSystem},
};
use crate::device::block::BlockDevice;

/// stand for file system
pub struct SuperBlockMeta {
    /// The device of the file system, None if it is a virtual file system
    device: Option<Arc<dyn BlockDevice>>,
    /// The file system
    file_system: Arc<dyn FileSystem>,
    /// The root of the file system, use weak to avoid reference cycle
    root: Once<Weak<dyn Dentry>>,
}

impl SuperBlockMeta {
    pub fn new(device: Option<Arc<dyn BlockDevice>>, file_system: Arc<dyn FileSystem>) -> Self {
        Self {
            device,
            file_system,
            root: Once::new(),
        }
    }
}

pub trait SuperBlock: Send + Sync + DowncastSync {
    fn meta(&self) -> &SuperBlockMeta;
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
