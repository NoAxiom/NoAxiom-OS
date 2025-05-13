use alloc::boxed::Box;
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;

use crate::{
    fs::vfs::basic::file::{File, FileMeta},
    include::io::PollEvent,
    syscall::{SysResult, SyscallResult},
};

pub struct CpuDmaLatencyFile {
    meta: FileMeta,
}

impl CpuDmaLatencyFile {
    pub fn new(meta: FileMeta) -> Self {
        Self { meta }
    }
}

#[async_trait]
impl File for CpuDmaLatencyFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!("readlink from zero")
    }
    async fn base_read(&self, _offset: usize, buf: &mut [u8]) -> SyscallResult {
        buf.fill(0);
        Ok(buf.len() as isize)
    }
    async fn base_write(&self, _offset: usize, buf: &[u8]) -> SyscallResult {
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
        unimplemented!("ZeroFile::poll not supported now");
    }
}
