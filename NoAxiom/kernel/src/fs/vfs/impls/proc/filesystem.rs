use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use driver::devices::block::BlockDevice;

use super::{init, superblock::ProcDevFsSuperBlock};
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

pub struct ProcDevFs {
    meta: FileSystemMeta,
}

impl ProcDevFs {
    pub fn new(name: &str) -> Self {
        Self {
            meta: FileSystemMeta::new(name),
        }
    }
    pub fn name() -> &'static str {
        "proc"
    }
}

#[async_trait]
impl FileSystem for ProcDevFs {
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
        let fs_super_block = Arc::new(ProcDevFsSuperBlock::new(super_block_meta));

        let root_dentry = Arc::new(RamFsDentry::new(
            parent.clone(),
            name,
            fs_super_block.clone(),
        ));
        let root_inode = Arc::new(RamFsDirInode::new(fs_super_block.clone(), 0));
        root_dentry.set_inode(root_inode);

        if let Some(parent) = parent.clone() {
            parent.add_child_directly(root_dentry.clone());
        }

        init(root_dentry.clone())
            .await
            .expect("proc fs init failed");
        root_dentry
    }
}
