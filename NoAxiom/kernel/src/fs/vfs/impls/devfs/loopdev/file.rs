use alloc::{boxed::Box, sync::Arc};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;
use kfuture::block::block_on;
use ksync::{mutex::SpinLock, AsyncMutex};

use crate::{
    cpu::current_task,
    fs::vfs::{
        basic::file::{File, FileMeta},
        impls::devfs::loopdev::{dentry::LoopDevDentry, inode::LoopDevInode},
    },
    include::{fs::FileFlags, io::PollEvent},
    mm::user_ptr::UserPtr,
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
    info: SpinLock<LoopInfo>,
    pub file: AsyncMutex<Option<Arc<dyn File>>>,
}

impl LoopDevFile {
    pub fn new(
        dentry: Arc<LoopDevDentry>,
        inode: Arc<LoopDevInode>,
        file_flags: &FileFlags,
        tar_file: Option<Arc<dyn File>>,
    ) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone(), file_flags),
            info: SpinLock::new(LoopInfo::new()),
            file: AsyncMutex::new(tar_file),
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
        let file_guard = self.file.lock().await;
        let file = file_guard.as_ref().ok_or(Errno::ENODEV)?;
        file.base_read(offset, buf).await
    }
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        let file_guard = self.file.lock().await;
        let file = file_guard.as_ref().ok_or(Errno::ENODEV)?;
        let inode = &self.meta.inode;
        let size = inode.size();
        if offset + buf.len() > size {
            inode.set_size(offset + buf.len());
        }
        let ret = file.base_write(offset, buf).await?;
        let mut loop_info = self.info.lock();
        loop_info.lo_offset += buf.len() as u64;
        Ok(ret)
    }
    async fn load_dir(&self) -> SysResult<()> {
        Err(Errno::ENOTDIR)
    }
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
    fn ioctl(&self, cmd: usize, arg: usize) -> SyscallResult {
        const LOOP_SET_FD: usize = 0x4C00;
        const LOOP_CLR_FD: usize = 0x4C01;
        const LOOP_SET_STATUS: usize = 0x4C02;
        const LOOP_GET_STATUS: usize = 0x4C03;
        let path = self.meta.dentry().path();
        info!("{} ioctl cmd: {:#x}, arg: {:#x}", &path, cmd, arg);
        let task = current_task().unwrap();
        match cmd {
            LOOP_SET_FD => {
                let file = task.fd_table().get(arg).ok_or(Errno::EBADF)?;
                self.file.spin_lock().replace(file);
                Ok(0)
            }
            LOOP_CLR_FD => {
                if self.file.spin_lock().is_some() {
                    self.file.spin_lock().take();
                } else {
                    return Err(Errno::ENXIO);
                }
                Ok(0)
            }
            LOOP_SET_STATUS => {
                let ptr = UserPtr::<LoopInfo>::new(arg);
                if ptr.is_null() {
                    return Err(Errno::EINVAL);
                }
                let info = block_on(ptr.read())?;
                *self.info.lock() = info;
                Ok(0)
            }
            LOOP_GET_STATUS => {
                let ptr = UserPtr::<LoopInfo>::new(arg);
                if ptr.is_null() {
                    return Err(Errno::EINVAL);
                }
                let loop_info = self.info.lock().clone();
                block_on(ptr.write(loop_info))?;
                Ok(0)
            }

            _ => Err(Errno::EINVAL),
        }
    }
    fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
        unimplemented!("LoopDev::poll not supported now");
    }
}
