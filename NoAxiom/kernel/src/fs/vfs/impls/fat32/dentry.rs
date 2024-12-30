use alloc::sync::Arc;

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
    nix::{fs::InodeMode, result::Errno},
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
}

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
}
