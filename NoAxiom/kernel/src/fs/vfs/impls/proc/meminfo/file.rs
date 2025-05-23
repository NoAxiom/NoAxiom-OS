use alloc::{boxed::Box, sync::Arc};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;

use crate::{
    fs::vfs::basic::file::{File, FileMeta},
    include::{fs::MemInfo, io::PollEvent},
    syscall::{SysResult, SyscallResult},
};

pub struct MemInfoFile {
    meta: FileMeta,
    meminfo: Arc<MemInfo>,
}

impl MemInfoFile {
    pub fn new(meta: FileMeta) -> Self {
        Self {
            meta,
            meminfo: Arc::new(MemInfo::new()),
        }
    }
}

#[async_trait]
impl File for MemInfoFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    #[allow(unused)]
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        // todo: maybe can just read empty
        return Ok(0);
        // let data = self.meminfo.serialize();
        // assert!(
        //     data.len() > offset,
        //     "data's len: {}, offset: {}",
        //     data.len(),
        //     offset
        // );
        // let len = core::cmp::min(data.len() - offset, buf.len());
        // buf[..len].copy_from_slice(&data.as_bytes()[offset..offset + len]);
        // Ok(len as isize)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        unreachable!("write to meminfo");
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
        unreachable!("MemInfoFile::poll not supported now");
    }
}
