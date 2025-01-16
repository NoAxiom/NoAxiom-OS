// ! **File** is the file system file instance opened in memory

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::sync::atomic::AtomicUsize;

use async_trait::async_trait;
use spin::Mutex;

use super::{dentry::Dentry, inode::Inode};
use crate::{
    nix::{fs::FileFlags, result::Errno},
    syscall::SyscallResult,
};

pub struct FileMeta {
    /// File flags, may be modified by multiple tasks
    flags: Mutex<FileFlags>,
    /// The position of the file, may be modified by multiple tasks
    pos: AtomicUsize,
    /// Pointer to the Dentry
    dentry: Arc<dyn Dentry>,
    /// Pointer to the Inode
    inode: Arc<dyn Inode>,
}

impl FileMeta {
    pub fn new(dent: Arc<dyn Dentry>, inode: Arc<dyn Inode>) -> Self {
        Self {
            flags: Mutex::new(FileFlags::empty()),
            pos: AtomicUsize::new(0),
            dentry: dent,
            inode,
        }
    }
    pub fn dentry(&self) -> Arc<dyn Dentry> {
        self.dentry.clone()
    }
}

#[async_trait]
pub trait File: Send + Sync {
    /// Get the size of file
    fn size(&self) -> usize {
        self.meta().inode.size()
    }
    /// Get the dentry of the file
    fn dentry(&self) -> Arc<dyn Dentry> {
        self.meta().dentry.clone()
    }
    /// Get the meta of the file
    fn meta(&self) -> &FileMeta;
    /// Read data from file at `offset` to `buf`
    async fn read_from<'a>(&'a self, offset: usize, buf: &'a mut Vec<u8>) -> SyscallResult;
    /// Write data to file at `offset` from `buf`
    async fn write_at<'a>(&'a self, offset: usize, buf: &'a Vec<u8>) -> SyscallResult;
    /// Load directory into memory, must be called before read/write explicitly,
    /// only for directories
    async fn load_dir(&self) -> Result<(), Errno>;
}

impl dyn File {
    pub async fn read_all(&self) -> Result<Vec<u8>, Errno> {
        let len = self.meta().inode.size();
        let mut buf = vec![0; len];
        self.read_from(0, &mut buf).await?;
        Ok(buf)
    }
    pub fn name(&self) -> String {
        self.dentry().name()
    }
}
