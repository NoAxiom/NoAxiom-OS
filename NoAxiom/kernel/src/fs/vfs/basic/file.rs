// ! **File** is the file system file instance opened in memory

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::sync::atomic::AtomicUsize;

use async_trait::async_trait;
type Mutex<T> = ksync::mutex::SpinLock<T>;

use core::sync::atomic::Ordering;

use super::{dentry::Dentry, inode::Inode};
use crate::{
    include::{fs::FileFlags, result::Errno},
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
    pub inode: Arc<dyn Inode>,
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
    pub fn readable(&self) -> bool {
        let flags = self.flags.lock();
        !flags.contains(FileFlags::O_WRONLY) || flags.contains(FileFlags::O_RDWR)
    }
    pub fn writable(&self) -> bool {
        let flags = self.flags.lock();
        flags.contains(FileFlags::O_WRONLY) || flags.contains(FileFlags::O_RDWR)
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
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult;
    /// Write data to file at `offset` from `buf`
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult;
    /// Load directory into memory, must be called before read/write explicitly,
    /// only for directories
    async fn load_dir(&self) -> Result<(), Errno>;
}

impl dyn File {
    pub async fn read_all(&self) -> Result<Vec<u8>, Errno> {
        let len = self.meta().inode.size();
        let mut buf = vec![0; len];
        self.base_read(0, &mut buf).await?;
        Ok(buf)
    }
    pub async fn read(&self, buf: &mut [u8]) -> SyscallResult {
        let offset = self.meta().pos.load(Ordering::Relaxed);
        let len = self.base_read(offset, buf).await?;
        self.meta().pos.fetch_add(len as usize, Ordering::Relaxed);
        Ok(len)
    }
    pub async fn write(&self, buf: &mut [u8]) -> SyscallResult {
        let offset = self.meta().pos.load(Ordering::Relaxed);
        let len = self.base_write(offset, buf).await?;
        self.meta().pos.fetch_add(len as usize, Ordering::Relaxed);
        Ok(len)
    }
    pub fn name(&self) -> String {
        self.dentry().name()
    }
    pub fn set_flags(&self, flags: FileFlags) {
        *self.meta().flags.lock() = flags;
    }
    pub fn inode(&self) -> Arc<dyn Inode> {
        self.meta().inode.clone()
    }
}
