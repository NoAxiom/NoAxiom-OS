use alloc::{boxed::Box, sync::Arc};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;

use crate::{
    fs::vfs::basic::file::{File, FileMeta},
    include::io::PollEvent,
    syscall::{SysResult, SyscallResult},
};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct LoopInfo {
    lo_device: u64,
    lo_inode: u64,
    lo_rdevice: u64,
    lo_offset: u64,
    lo_sizelimit: u64,
    lo_number: u32,
    lo_encrypt_type: u32,
    lo_encrypt_key_size: u32,
    lo_flags: u32,
    lo_encrypt_key: [u8; 32],
    lo_init: [u64; 2],
}

impl LoopInfo {
    pub fn new() -> Self {
        Self {
            lo_device: 0,
            lo_inode: 0,
            lo_rdevice: 0,
            lo_offset: 0,
            lo_sizelimit: 0,
            lo_number: 0,
            lo_encrypt_type: 0,
            lo_encrypt_key_size: 0,
            lo_flags: 0,
            lo_encrypt_key: [0; 32],
            lo_init: [0; 2],
        }
    }
}

pub struct LoopDevFile {
    meta: FileMeta,
    info: LoopInfo,
    file: Arc<dyn File>,
}

impl LoopDevFile {
    pub fn new(meta: FileMeta, file: Arc<dyn File>) -> Self {
        Self {
            meta,
            info: LoopInfo::new(),
            file,
        }
    }
}

#[async_trait]
impl File for LoopDevFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!("readlink from LoopDev")
    }
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        self.file.base_read(offset, buf).await
    }
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        self.file.base_write(offset, buf).await
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
        unimplemented!("LoopDev::poll not supported now");
    }
}
