use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use driver::block::BlockDevice;

use super::{init, superblock::DevFsSuperBlock};
use crate::{
    fs::vfs::{
        basic::{
            dentry::Dentry,
            filesystem::{FileSystem, FileSystemMeta},
            superblock::SuperBlockMeta,
        },
        impls::ramfs::{dentry::RamFsDentry, inode::RamFsDirInode},
    },
    include::fs::MountFlags,
};

pub struct DevFs {
    meta: FileSystemMeta,
}

impl DevFs {
    pub fn new(name: &str) -> Self {
        Self {
            meta: FileSystemMeta::new(name),
        }
    }
    pub fn name() -> &'static str {
        "dev"
    }
}

#[async_trait]
impl FileSystem for DevFs {
    fn meta(&self) -> &FileSystemMeta {
        &self.meta
    }

    async fn root(
        self: Arc<Self>,
        parent: Option<Arc<dyn Dentry>>,
        _flags: MountFlags,
        name: &str,
        device: Option<&'static dyn BlockDevice>,
    ) -> Arc<dyn Dentry> {
        let super_block_meta = SuperBlockMeta::new(device, self.clone());
        let fs_super_block = Arc::new(DevFsSuperBlock::new(super_block_meta));

        let root_dentry = Arc::new(RamFsDentry::new(
            parent.clone(),
            name,
            fs_super_block.clone(),
        ));
        let root_inode = Arc::new(RamFsDirInode::new(fs_super_block.clone(), 0));
        root_dentry.into_dyn().set_inode(root_inode);

        if let Some(parent) = parent.clone() {
            parent.add_child(root_dentry.clone());
        }

        init(root_dentry.clone())
            .await
            .expect("proc fs init failed");
        root_dentry
    }
}
