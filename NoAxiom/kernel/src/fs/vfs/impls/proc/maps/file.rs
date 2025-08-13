use alloc::{boxed::Box, string::String, sync::Arc};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;

use crate::{
    cpu::current_task,
    fs::vfs::basic::file::{File, FileMeta},
    include::io::PollEvent,
    syscall::{SysResult, SyscallResult},
    task::Task,
};

fn get_maps(task: &Arc<Task>) -> String {
    String::new()
}

pub struct MapsFile {
    meta: FileMeta,
}

impl MapsFile {
    pub fn new(meta: FileMeta) -> Self {
        Self { meta }
    }
}

#[async_trait]
impl File for MapsFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }

    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let content = get_maps(current_task().expect("must have current task"));

        debug!(
            "[MapsFile::base_read]: offset: {}, content: {:?}",
            offset, content
        );

        let bytes = content.as_bytes();
        let content_len = bytes.len();
        if offset >= content_len {
            return Ok(0);
        }

        let len = buf.len().min(content_len - offset);
        buf[..len].copy_from_slice(&bytes[offset..offset + len]);

        Ok(len as isize)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        unreachable!("write to MapsFile");
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
        unreachable!("MapsFile::poll not supported now");
    }
}
