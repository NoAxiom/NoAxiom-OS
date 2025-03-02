use alloc::{boxed::Box, sync::Arc, vec::Vec};

use async_trait::async_trait;
type Mutex<T> = ksync::mutex::SpinLock<T>;

use super::{
    dentry::FAT32Dentry,
    inode::{FAT32DirInode, FAT32FileInode},
};
use crate::{
    fs::{
        fat32::{
            directory::FAT32Directory as FAT32FIleSystemDirectory,
            file::FAT32File as FAT32FIleSystemFile,
        },
        vfs::basic::file::{File, FileMeta},
    },
    include::{fs::InodeMode, result::Errno},
    syscall::SyscallResult,
};

pub struct FAT32File {
    meta: FileMeta,
    file: Arc<Mutex<FAT32FIleSystemFile>>,
}

impl FAT32File {
    pub fn new(dentry: Arc<FAT32Dentry>, inode: Arc<FAT32FileInode>) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone()),
            file: inode.get_file(),
        }
    }
}

#[async_trait]
impl File for FAT32File {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let file_size = self.size();

        if offset >= file_size {
            return Err(Errno::EINVAL);
        }

        if offset + buf.len() > file_size {
            warn!("Read buffer is too large, resize it to fit the file size");
            // buf.resize(file_size - offset, 0);
        }

        self.file.lock().read_from(offset, buf).await
    }
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        let file_size = self.size();

        if offset >= file_size {
            return Err(Errno::EINVAL);
        }

        if offset + buf.len() > file_size {
            panic!("store data too long!");
        }

        self.file.lock().write_at(offset, buf).await
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
}

pub struct FAT32Directory {
    meta: FileMeta,
    file: Arc<Mutex<FAT32FIleSystemDirectory>>,
}

impl FAT32Directory {
    pub fn new(dentry: Arc<FAT32Dentry>, inode: Arc<FAT32DirInode>) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone()),
            file: inode.get_dir(),
        }
    }
}

#[async_trait]
impl File for FAT32Directory {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let _ = offset;
        let _ = buf;
        Err(Errno::ENOSYS)
    }
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        let _ = offset;
        let _ = buf;
        Err(Errno::ENOSYS)
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        let parent = self.dentry();

        let dir_files = self.file.lock().load().await;

        for dir_file in dir_files {
            let name = dir_file.name();
            let file_type = dir_file.file_type();
            match file_type {
                InodeMode::DIR => {
                    let dir = dir_file.downcast_ref::<FAT32FIleSystemDirectory>().unwrap();
                    let super_block = self.meta().dentry().super_block();
                    // ! todo: is that clone too expensive??
                    let child_inode = FAT32DirInode::new(super_block, dir.clone());
                    parent.add_child(&name, Arc::new(child_inode));
                }
                InodeMode::FILE => {
                    let file = dir_file.downcast_ref::<FAT32FIleSystemFile>().unwrap();
                    let super_block = self.meta().dentry().super_block();
                    let child_inode = FAT32FileInode::new(super_block, file.clone());
                    parent.add_child(&name, Arc::new(child_inode));
                }
                _ => {
                    panic!("Unsupported file type");
                }
            };
        }
        Ok(())
    }
}
