use alloc::boxed::Box;

use async_trait::async_trait;
use include::errno::Errno;

use crate::{
    fs::vfs::basic::file::{File, FileMeta},
    syscall::{SysResult, SyscallResult},
};

pub struct NullFile {
    meta: FileMeta,
}

impl NullFile {
    pub fn new(meta: FileMeta) -> Self {
        Self { meta }
    }
}

#[async_trait]
impl File for NullFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }

    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!("readlink from null");
    }

    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        Ok(0)
    }

    async fn base_write(&self, _offset: usize, buf: &[u8]) -> SyscallResult {
        Ok(buf.len() as isize)
    }

    async fn load_dir(&self) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
}
