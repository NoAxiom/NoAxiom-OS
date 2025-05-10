use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use driver::devices::impls::block::BlockDevice;
use ext4_rs::ext4_defs::ROOT_INODE;
use spin::Mutex;

use super::{dentry::Ext4Dentry, inode::Ext4DirInode, superblock::Ext4SuperBlock, IExtFs};
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

pub struct AsyncSmpExt4 {
    meta: FileSystemMeta,
}

impl AsyncSmpExt4 {
    pub fn new(name: &str) -> Self {
        Self {
            meta: FileSystemMeta::new(name),
        }
    }
    pub fn name() -> &'static str {
        "ext4"
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
        device: Option<Arc<&'static dyn BlockDevice>>,
    ) -> Arc<dyn Dentry> {
        let super_block_meta = SuperBlockMeta::new(device.clone(), self.clone());
        let blk = device.unwrap();
        let disk_cursor = DiskCursor::new(blk, 0, 0);
        let unbooted_fs = Arc::new(Mutex::new(IExtFs::open(Arc::new(disk_cursor)).await));
        let fs_super_block = Arc::new(Ext4SuperBlock::new(super_block_meta, unbooted_fs));

        let root_dentry = Arc::new(Ext4Dentry::new(
            parent.clone(),
            name,
            fs_super_block.clone(),
        ));

        let ext4 = fs_super_block.get_fs();

        let root_inode = Arc::new(Ext4DirInode::new(
            fs_super_block.clone(),
            ext4.get_inode_ref(ROOT_INODE).await,
        ));
        root_dentry.set_inode(root_inode);

        if let Some(parent) = parent {
            parent.add_child_directly(root_dentry.clone());
        }
        root_dentry
    }
}
