use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;

use super::{
    file::{RamFsDir, RamFsFile},
    inode::{RamFsDirInode, RamFsFileInode},
};
use crate::{
    fs::vfs::basic::{
        dentry::{Dentry, DentryMeta},
        file::File,
        inode::Inode,
        superblock::SuperBlock,
    },
    include::{fs::InodeMode, result::Errno},
    syscall::SysResult,
};

pub struct RamFsDentry {
    meta: DentryMeta,
}

impl RamFsDentry {
    pub fn new(
        parent: Option<Arc<dyn Dentry>>,
        name: &str,
        super_block: Arc<dyn SuperBlock>,
    ) -> Self {
        Self {
            meta: DentryMeta::new(parent, name, super_block),
        }
    }
    fn into_dyn(self: Arc<Self>) -> Arc<dyn Dentry> {
        self.clone()
    }
}

#[async_trait]
impl Dentry for RamFsDentry {
    fn meta(&self) -> &DentryMeta {
        &self.meta
    }

    fn from_name(self: Arc<Self>, name: &str) -> Arc<dyn Dentry> {
        let super_block = self.meta.super_block.clone();
        Arc::new(Self::new(Some(self), name, super_block))
    }

    fn open(self: Arc<Self>) -> SysResult<Arc<dyn File>> {
        let inode = self.inode()?;
        match inode.file_type() {
            InodeMode::DIR => Ok(Arc::new(RamFsDir::new(
                self.clone(),
                inode
                    .downcast_arc::<RamFsDirInode>()
                    .map_err(|_| Errno::EIO)?,
            ))),
            InodeMode::FILE => Ok(Arc::new(RamFsFile::new(
                self.clone(),
                inode
                    .downcast_arc::<RamFsFileInode>()
                    .map_err(|_| Errno::EIO)?,
            ))),
            InodeMode::SOCKET => Ok(Arc::new(RamFsFile::new(
                self.clone(),
                inode
                    .downcast_arc::<RamFsFileInode>()
                    .map_err(|_| Errno::EIO)?,
            ))),
            _ => unreachable!(),
        }
    }

    async fn create(self: Arc<Self>, name: &str, mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        let super_block = self.meta.super_block.clone();

        let sub_inode: Arc<dyn Inode> = if mode.contains(InodeMode::FILE) {
            Arc::new(RamFsFileInode::new(super_block, 0))
        } else if mode.contains(InodeMode::DIR) {
            Arc::new(RamFsDirInode::new(super_block, 0))
        } else if mode.contains(InodeMode::SOCKET) {
            Arc::new(RamFsFileInode::new(super_block, 0))
        } else {
            unreachable!("create unknown inode type")
        };

        Ok(self.into_dyn().add_child(name, sub_inode))
    }
}
