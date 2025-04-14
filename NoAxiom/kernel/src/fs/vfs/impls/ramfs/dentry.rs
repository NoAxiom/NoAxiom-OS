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

    /// Just create. Don't like real fs that link to its parent
    async fn create(self: Arc<Self>, name: &str, mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        let super_block = self.meta.super_block.clone();
        let sub_dentry = self.from_name(name);
        let sub_inode: Arc<dyn Inode> = match mode {
            InodeMode::DIR => Arc::new(RamFsDirInode::new(super_block, 0)),
            InodeMode::FILE => Arc::new(RamFsFileInode::new(super_block, 0)),
            InodeMode::SOCKET => Arc::new(RamFsFileInode::new(super_block, 0)),
            _ => return Err(Errno::EPERM),
        };
        sub_dentry.set_inode(sub_inode);
        Ok(sub_dentry)
    }
}
