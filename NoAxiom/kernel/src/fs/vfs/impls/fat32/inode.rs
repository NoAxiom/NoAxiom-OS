use alloc::sync::Arc;

type Mutex<T> = ksync::mutex::SpinLock<T>;
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
    include::fs::{InodeMode, Stat},
};

pub struct FAT32FileInode {
    meta: InodeMeta,
    pub file: Arc<Mutex<FAT32FIleSystemFile>>,
}

impl FAT32FileInode {
    pub fn new(superblock: Arc<dyn superblock::SuperBlock>, file: FAT32FIleSystemFile) -> Self {
        Self {
            meta: InodeMeta::new(superblock, InodeMode::FILE, file.size()),
            file: Arc::new(Mutex::new(file)),
        }
    }
    pub fn get_file(&self) -> Arc<Mutex<FAT32FIleSystemFile>> {
        self.file.clone()
    }
}

impl Inode for FAT32FileInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::include::fs::Stat, crate::include::result::Errno> {
        let inner = self.meta.inner.lock();
        let mode = self.meta.inode_mode.bits();
        let len = inner.size;
        Ok(Stat {
            st_dev: 0,
            st_ino: self.meta.id as u64,
            st_mode: mode,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: len as u64,
            st_blksize: 512,
            __pad2: 0,
            st_blocks: (len as u64 / 512) as u64,
            st_atime_sec: inner.atime_sec as u64,
            st_atime_nsec: inner.atime_nsec as u64,
            st_mtime_sec: inner.mtime_sec as u64,
            st_mtime_nsec: inner.mtime_nsec as u64,
            st_ctime_sec: inner.ctime_sec as u64,
            st_ctime_nsec: inner.ctime_nsec as u64,
            unused: 0,
        })
    }
}

pub struct FAT32DirInode {
    meta: InodeMeta,
    pub file: Arc<Mutex<FAT32FIleSystemDirectory>>,
}

impl FAT32DirInode {
    pub fn new(
        superblock: Arc<dyn superblock::SuperBlock>,
        directory: FAT32FIleSystemDirectory,
    ) -> Self {
        Self {
            meta: InodeMeta::new(superblock, InodeMode::DIR, 0),
            file: Arc::new(Mutex::new(directory)),
        }
    }
    pub fn get_dir(&self) -> Arc<Mutex<FAT32FIleSystemDirectory>> {
        self.file.clone()
    }
}

impl Inode for FAT32DirInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> Result<crate::include::fs::Stat, crate::include::result::Errno> {
        let inner = self.meta.inner.lock();
        let mode = self.meta.inode_mode.bits();
        Ok(Stat {
            st_dev: 0,
            st_ino: self.meta.id as u64,
            st_mode: mode,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: 0,
            st_blksize: 512,
            __pad2: 0,
            st_blocks: 0,
            st_atime_sec: inner.atime_sec as u64,
            st_atime_nsec: inner.atime_nsec as u64,
            st_mtime_sec: inner.mtime_sec as u64,
            st_mtime_nsec: inner.mtime_nsec as u64,
            st_ctime_sec: inner.ctime_sec as u64,
            st_ctime_nsec: inner.ctime_nsec as u64,
            unused: 0,
        })
    }
}
