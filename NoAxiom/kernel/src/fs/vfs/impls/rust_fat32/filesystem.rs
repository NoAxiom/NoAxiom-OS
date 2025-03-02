use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;

use super::{
    dentry::Fat32Dentry, disk_cursor::DiskCursor, inode::Fat32DirInode,
    superblock::Fat32SuperBlock, IFatFs,
};
use crate::{
    device::block::BlockDevice,
    fs::vfs::basic::{
        dentry::Dentry,
        filesystem::{FileSystem, FileSystemMeta},
        superblock::SuperBlockMeta,
    },
    include::fs::MountFlags,
};

pub struct AsyncSmpFat32 {
    meta: FileSystemMeta,
}

impl AsyncSmpFat32 {
    pub fn new(name: &str) -> Self {
        Self {
            meta: FileSystemMeta::new(name),
        }
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
        device: Option<Arc<dyn BlockDevice>>,
    ) -> Arc<dyn Dentry> {
        let super_block_meta = SuperBlockMeta::new(device.clone(), self.clone());
        let unbooted_fs = Arc::new(
            IFatFs::new(
                DiskCursor::new(device.unwrap(), 0, 0),
                fatfs::FsOptions::new(),
            )
            .await
            .unwrap(),
        );
        let fs_super_block = Arc::new(Fat32SuperBlock::new(super_block_meta, unbooted_fs));

        let root_dentry = Fat32Dentry::new(parent, name, fs_super_block.clone());
        let root_inode = Fat32DirInode::new(
            fs_super_block.clone(),
            fs_super_block.clone().inner.root_dir(),
        );

        root_dentry.set_inode(Arc::new(root_inode));
        Arc::new(root_dentry)
    }
}
