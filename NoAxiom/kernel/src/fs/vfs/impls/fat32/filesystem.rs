use alloc::{boxed::Box, string::String, sync::Arc};

use super::{dentry::FAT32Dentry, inode::FAT32DirInode, superblock::FAT32SuperBlock};
use crate::{
    device::block::BlockDevice,
    fs::{
        fat32::FAT32FIleSystem as FAT32FileSystemSpecific,
        vfs::basic::{
            dentry::Dentry,
            filesystem::{FileSystem, FileSystemMeta},
            superblock::SuperBlockMeta,
        },
    },
    include::fs::MountFlags,
};

pub struct FAT32FIleSystem {
    meta: FileSystemMeta,
}

impl FAT32FIleSystem {
    pub fn new(name: &str) -> Self {
        Self {
            meta: FileSystemMeta::new(name),
        }
    }
}

#[async_trait::async_trait]
impl FileSystem for FAT32FIleSystem {
    fn meta(&self) -> &FileSystemMeta {
        &self.meta
    }

    async fn root(
        self: Arc<Self>,
        parent: Option<Arc<dyn Dentry>>,
        flags: MountFlags,
        name: &str,
        device: Option<Arc<dyn BlockDevice>>,
    ) -> Arc<dyn Dentry> {
        let unbooted_fs = Arc::new(FAT32FileSystemSpecific::new(device.clone().unwrap()));
        let super_block_meta = SuperBlockMeta::new(device.clone(), self.clone());
        let fs_super_block = Arc::new(FAT32SuperBlock::new(super_block_meta, unbooted_fs));
        let dentry = FAT32Dentry::new(parent, name, fs_super_block.clone());
        let root = FAT32FileSystemSpecific::load_root(device.unwrap()).await;
        let root_inode = FAT32DirInode::new(fs_super_block, root);

        dentry.set_inode(Arc::new(root_inode));
        Arc::new(dentry)
    }
}
