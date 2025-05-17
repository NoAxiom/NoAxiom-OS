use alloc::boxed::Box;
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;

use crate::{
    cpu::current_task,
    fs::vfs::basic::file::{File, FileMeta},
    include::io::PollEvent,
    syscall::{SysResult, SyscallResult},
};

pub struct ExeFile {
    meta: FileMeta,
}

impl ExeFile {
    pub fn new(meta: FileMeta) -> Self {
        Self { meta }
    }
}

#[async_trait]
impl File for ExeFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }

    async fn base_readlink(&self, buf: &mut [u8]) -> SyscallResult {
        let exe = current_task().unwrap().cwd().as_string();
        if buf.len() < exe.len() + 1 {
            warn!("readlink buf not big enough");
            return Err(Errno::EINVAL);
        }
        buf[0..exe.len()].copy_from_slice(exe.as_bytes());
        buf[exe.len()] = '\0' as u8;
        Ok((exe.len() + 1) as isize)
    }

    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        unreachable!("read from exe");
    }

    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        unreachable!("write to exe");
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
        unreachable!("ExeFile::poll not supported now");
    }
}
