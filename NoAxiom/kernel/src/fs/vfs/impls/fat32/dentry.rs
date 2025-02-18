use alloc::{boxed::Box, string::ToString, sync::Arc};

use async_trait::async_trait;

use super::{
    file::{FAT32Directory, FAT32File},
    inode::{FAT32DirInode, FAT32FileInode},
};
use crate::{
    fs::vfs::basic::{
        dentry::{Dentry, DentryMeta},
        file::File,
        superblock::SuperBlock,
    },
    include::{fs::InodeMode, result::Errno},
};

pub struct FAT32Dentry {
    meta: DentryMeta,
}

impl FAT32Dentry {
    pub fn new(
        parent: Option<Arc<dyn Dentry>>,
        name: &str,
        super_block: Arc<dyn SuperBlock>,
    ) -> Self {
        Self {
            meta: DentryMeta::new(parent, name, super_block),
        }
    }

    pub fn into_dyn_dentry(self: Arc<Self>) -> Arc<dyn Dentry> {
        self.clone()
    }
}

#[async_trait]
impl Dentry for FAT32Dentry {
    fn meta(&self) -> &DentryMeta {
        &self.meta
    }

    fn from_name(self: Arc<Self>, name: &str) -> Arc<dyn Dentry> {
        let super_block = self.meta.super_block.clone();
        Arc::new(Self::new(Some(self), name, super_block))
    }

    fn open(self: Arc<Self>) -> Result<Arc<dyn File>, Errno> {
        let inode = self.inode()?;
        match inode.file_type() {
            InodeMode::DIR => Ok(Arc::new(FAT32Directory::new(
                self.clone(),
                inode
                    .downcast_arc::<FAT32DirInode>()
                    .map_err(|_| Errno::EIO)?,
            ))),
            InodeMode::FILE => Ok(Arc::new(FAT32File::new(
                self.clone(),
                inode
                    .downcast_arc::<FAT32FileInode>()
                    .map_err(|_| Errno::EIO)?,
            ))),
            _ => Err(Errno::EINVAL),
        }
    }

    async fn create(
        self: Arc<Self>,
        name: &str,
        mode: InodeMode,
    ) -> Result<Arc<dyn Dentry>, Errno> {
        let inode = self.inode()?;
        let super_block = &self.meta().super_block;
        assert!(inode.file_type() == InodeMode::DIR);

        if mode.contains(InodeMode::FILE) {
            let dir = inode
                .downcast_arc::<FAT32DirInode>()
                .map_err(|_| Errno::EIO)?;
            // todo: update size
            let new_file = dir.get_dir().lock().create_file(name.to_string(), 1).await;
            let new_inode = FAT32FileInode::new(super_block.clone(), new_file);
            let new_dentry = self.into_dyn_dentry().new_child(name);
            new_dentry.set_inode(Arc::new(new_inode));
            Ok(new_dentry)
        } else if mode.contains(InodeMode::DIR) {
            let dir = inode
                .downcast_arc::<FAT32DirInode>()
                .map_err(|_| Errno::EIO)?;
            // todo: update size
            let new_dir = dir.get_dir().lock().create_dir(name.to_string(), 1).await;
            let new_inode = FAT32DirInode::new(super_block.clone(), new_dir);
            let new_dentry = self.into_dyn_dentry().new_child(name);
            new_dentry.set_inode(Arc::new(new_inode));
            Ok(new_dentry)
        } else {
            Err(Errno::EINVAL)
        }
    }
}
