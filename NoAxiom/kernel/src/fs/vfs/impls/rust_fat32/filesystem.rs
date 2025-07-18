use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use driver::devices::block::BlockDevice;

use super::{dentry::Fat32Dentry, inode::Fat32DirInode, superblock::Fat32SuperBlock, IFatFs};
use crate::{
    fs::vfs::{
        basic::{
            dentry::Dentry,
            filesystem::{FileSystem, FileSystemMeta},
            superblock::SuperBlockMeta,
        },
        impls::disk_cursor::DiskCursor,
    },
    include::fs::MountFlags,
};

pub struct AsyncSmpFat32 {
    meta: FileSystemMeta,
}

impl AsyncSmpFat32 {
    #[allow(unused)]
    pub fn new(name: &str) -> Self {
        Self {
            meta: FileSystemMeta::new(name),
        }
    }
    pub fn name() -> &'static str {
        "vfat"
    }
}

#[async_trait]
impl FileSystem for AsyncSmpFat32 {
    fn meta(&self) -> &FileSystemMeta {
        &self.meta
    }

    async fn root(
        self: Arc<Self>,
        parent: Option<Arc<dyn Dentry>>,
        _flags: MountFlags,
        name: &str,
        device: Option<Arc<&'static dyn BlockDevice>>,
    ) -> Arc<dyn Dentry> {
        let super_block_meta = SuperBlockMeta::new(device.clone(), self.clone());
        let blk = device.unwrap();
        let unbooted_fs = Arc::new(
            IFatFs::new(DiskCursor::new(blk, 0, 0), fatfs::FsOptions::new())
                .await
                .unwrap(),
        );
        let fs_super_block = Arc::new(Fat32SuperBlock::new(super_block_meta, unbooted_fs));

        let root_dentry = Arc::new(Fat32Dentry::new(
            parent.clone(),
            name,
            fs_super_block.clone(),
        ));
        let root_inode = Arc::new(Fat32DirInode::new(
            fs_super_block.clone(),
            fs_super_block.clone().inner.root_dir(),
        ));
        root_dentry.set_inode(root_inode);

        if let Some(parent) = parent {
            parent.add_child_directly(root_dentry.clone());
        }

        root_dentry
    }
}
