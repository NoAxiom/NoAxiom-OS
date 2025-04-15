use alloc::boxed::Box;

use async_trait::async_trait;
use include::errno::Errno;

use crate::{
    fs::{
        manager::FS_MANAGER,
        vfs::basic::file::{File, FileMeta},
    },
    syscall::{SysResult, SyscallResult},
};

pub struct MountsFile {
    meta: FileMeta,
}

impl MountsFile {
    pub fn new(meta: FileMeta) -> Self {
        Self { meta }
    }
}

#[async_trait]
impl File for MountsFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }

    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        // todo: maybe can just read empty
        let data = FS_MANAGER.get_list();
        let data = data.join("\n");
        assert!(data.len() > offset);
        let len = core::cmp::min(data.len() - offset, buf.len());
        buf[..len].copy_from_slice(&data.as_bytes()[offset..offset + len]);
        Ok(len as isize)
    }

    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }

    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        unreachable!("write to mountsfile");
    }

    async fn load_dir(&self) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
    }
}
