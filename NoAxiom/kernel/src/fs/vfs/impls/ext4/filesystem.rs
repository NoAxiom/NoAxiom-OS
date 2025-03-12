use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use ext4_rs::ext4_defs::ROOT_INODE;
use spin::Mutex;

use super::{dentry::Ext4Dentry, inode::Ext4DirInode, superblock::Ext4SuperBlock, IExtFs};
use crate::{
    device::block::BlockDevice,
    fs::{
        blockcache::AsyncBlockCache,
        vfs::basic::{
            dentry::Dentry,
            filesystem::{FileSystem, FileSystemMeta},
            superblock::SuperBlockMeta,
        },
    },
    include::fs::MountFlags,
};

pub struct AsyncSmpExt4 {
    meta: FileSystemMeta,
}

impl AsyncSmpExt4 {
    pub fn new(name: &str) -> Self {
        Self {
            meta: FileSystemMeta::new(name),
        }
    }
}

#[async_trait]
impl FileSystem for AsyncSmpExt4 {
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
        let blk = Arc::new(AsyncBlockCache::from(device.unwrap()));
        let unbooted_fs = Arc::new(Mutex::new(IExtFs::open(blk).await));
        let fs_super_block = Arc::new(Ext4SuperBlock::new(super_block_meta, unbooted_fs));

        let root_dentry = Ext4Dentry::new(parent.clone(), name, fs_super_block.clone());

        let ext4 = fs_super_block.get_fs();

        let root_inode = Arc::new(Ext4DirInode::new(
            fs_super_block.clone(),
            ext4.get_inode_ref(ROOT_INODE).await,
        ));

        if let Some(parent) = parent {
            parent.add_child(name, root_inode.clone());
        }

        root_dentry.set_inode(root_inode);
        Arc::new(root_dentry)
    }
}
