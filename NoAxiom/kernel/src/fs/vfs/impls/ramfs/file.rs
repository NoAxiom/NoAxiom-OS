use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;
use ksync::mutex::RwLock;

use super::{
    dentry::RamFsDentry,
    inode::{RamFsDirInode, RamFsFileInode},
};
use crate::{
    fs::vfs::basic::file::{File, FileMeta},
    include::io::PollEvent,
    syscall::{SysResult, SyscallResult},
};

pub struct RamFsFile {
    meta: FileMeta,
    data: Arc<RwLock<Vec<u8>>>,
}

impl RamFsFile {
    pub fn new(dentry: Arc<RamFsDentry>, inode: Arc<RamFsFileInode>) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone()),
            data: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl File for RamFsFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let data = self.data.read();
        assert!(data.len() > offset);
        let len = core::cmp::min(data.len() - offset, buf.len());
        buf[..len].copy_from_slice(&data[offset..offset + len]);
        Ok(len as isize)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        let mut data = self.data.write();
        if offset + buf.len() > data.len() {
            data.resize(offset + buf.len(), 0);
        }
        let dst = &mut data[offset..offset + buf.len()];
        dst.copy_from_slice(&buf[..dst.len()]);
        Ok(buf.len() as isize)
    }
    async fn load_dir(&self) -> SysResult<()> {
        Err(Errno::ENOTDIR)
    }
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
    }
    fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
        unreachable!("RamfsFile::poll not supported now");
    }
}

pub struct RamFsDir {
    meta: FileMeta,
}

impl RamFsDir {
    pub fn new(dentry: Arc<RamFsDentry>, inode: Arc<RamFsDirInode>) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone()),
        }
    }
}

#[async_trait]
impl File for RamFsDir {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        Err(Errno::EISDIR)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        Err(Errno::EISDIR)
    }
    async fn load_dir(&self) -> SysResult<()> {
        Ok(())
    }
    /// Ram fs does not really has child or parent
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Ok(())
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
    }
    fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
        unreachable!("RamfsDir::poll not supported now");
    }
}
