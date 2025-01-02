use alloc::sync::Arc;

use ksync::mutex::SpinLock;

use crate::{
    fs::{
        fat32::{
            directory::FAT32Directory as FAT32FIleSystemDirectory,
            file::FAT32File as FAT32FIleSystemFile,
        },
        vfs::basic::{
            inode::{Inode, InodeMeta},
            superblock,
        },
    },
    nix::fs::InodeMode,
};

pub struct FAT32FileInode {
    meta: InodeMeta,
    pub file: Arc<SpinLock<FAT32FIleSystemFile>>,
}

impl FAT32FileInode {
    pub fn new(superblock: Arc<dyn superblock::SuperBlock>, file: FAT32FIleSystemFile) -> Self {
        Self {
            meta: InodeMeta::new(superblock, InodeMode::FILE, file.size()),
            file: Arc::new(SpinLock::new(file)),
        }
    }
    pub fn get_file(&self) -> Arc<SpinLock<FAT32FIleSystemFile>> {
        self.file.clone()
    }
}

impl Inode for FAT32FileInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::nix::fs::Stat, crate::nix::result::Errno> {
        todo!()
    }
}

pub struct FAT32DirInode {
    meta: InodeMeta,
    pub file: Arc<SpinLock<FAT32FIleSystemDirectory>>,
}

impl FAT32DirInode {
    pub fn new(
        superblock: Arc<dyn superblock::SuperBlock>,
        directory: FAT32FIleSystemDirectory,
    ) -> Self {
        Self {
            meta: InodeMeta::new(superblock, InodeMode::DIR, 0),
            file: Arc::new(SpinLock::new(directory)),
        }
    }
    pub fn get_dir(&self) -> Arc<SpinLock<FAT32FIleSystemDirectory>> {
        self.file.clone()
    }
}

impl Inode for FAT32DirInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::nix::fs::Stat, crate::nix::result::Errno> {
        todo!()
    }
}
