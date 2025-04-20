use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;

use super::{
    file::{Fat32Dir, Fat32File},
    inode::{Fat32DirInode, Fat32FileInode},
};
use crate::{
    fs::vfs::{
        basic::{
            dentry::{Dentry, DentryMeta},
            file::File,
            superblock::SuperBlock,
        },
        impls::rust_fat32::fs_err,
    },
    include::{fs::InodeMode, result::Errno},
};

pub struct Fat32Dentry {
    meta: DentryMeta,
}

impl Fat32Dentry {
    pub fn new(
        parent: Option<Arc<dyn Dentry>>,
        name: &str,
        super_block: Arc<dyn SuperBlock>,
    ) -> Self {
        Self {
            meta: DentryMeta::new(parent, name, super_block),
        }
    }

    pub fn into_dyn(self: Arc<Self>) -> Arc<dyn Dentry> {
        self.clone()
    }
}

#[async_trait]
impl Dentry for Fat32Dentry {
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
            InodeMode::DIR => Ok(Arc::new(Fat32Dir::new(
                self.clone(),
                inode
                    .downcast_arc::<Fat32DirInode>()
                    .map_err(|_| Errno::EIO)?,
            ))),
            InodeMode::FILE => Ok(Arc::new(Fat32File::new(
                self.clone(),
                inode
                    .downcast_arc::<Fat32FileInode>()
                    .map_err(|_| Errno::EIO)?,
            ))),
            _ => Err(Errno::EINVAL),
        }
    }

    async fn create(self: Arc<Self>, name: &str, mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        let inode = self.inode()?;
        let super_block = &self.meta().super_block;
        assert!(inode.file_type() == InodeMode::DIR);
        if mode.contains(InodeMode::FILE) {
            assert!(ksync::mutex::check_no_lock());
            let dir = inode
                .downcast_arc::<Fat32DirInode>()
                .map_err(|_| Errno::EIO)?;
            assert!(ksync::mutex::check_no_lock());
            let new_file = dir
                .get_dir()
                .lock()
                .create_file(name)
                .await
                .map_err(fs_err)?;
            let new_inode = Fat32FileInode::new(super_block.clone(), new_file);
            Ok(self.into_dyn().add_child(name, Arc::new(new_inode)))
        } else if mode.contains(InodeMode::DIR) {
            assert!(ksync::mutex::check_no_lock());
            let dir = inode
                .downcast_arc::<Fat32DirInode>()
                .map_err(|_| Errno::EIO)?;
            assert!(ksync::mutex::check_no_lock());
            debug!("create dir: {}", name);
            let new_dir = dir
                .get_dir()
                .lock()
                .create_dir(name)
                .await
                .map_err(fs_err)?;
            debug!("create dir ok");
            let new_inode = Fat32DirInode::new(super_block.clone(), new_dir);
            Ok(self.into_dyn().add_child(name, Arc::new(new_inode)))
        } else {
            Err(Errno::EINVAL)
        }
    }
}
