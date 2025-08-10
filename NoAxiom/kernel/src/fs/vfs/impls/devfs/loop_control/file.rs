use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;
use ksync::mutex::{SpinLock, SpinLockGuard};
use lazy_static::lazy_static;

use crate::{
    fs::{
        path::kopen,
        vfs::{
            basic::{
                dentry::Dentry,
                file::{File, FileMeta},
            },
            impls::devfs::loopdev::{
                dentry::LoopDevDentry, file::LoopDevFile, inode::LoopDevInode,
            },
        },
    },
    include::{fs::FileFlags, io::PollEvent},
    syscall::{SysResult, SyscallResult},
};

lazy_static! {
    static ref LOOP_CONTROL: SpinLock<LoopDevManager> = SpinLock::new(LoopDevManager::new());
}

pub fn get_loop_control() -> SpinLockGuard<'static, LoopDevManager> {
    LOOP_CONTROL.lock()
}

pub struct LoopDevManager {
    loops: Vec<Arc<LoopDevFile>>,
}

impl LoopDevManager {
    fn new() -> Self {
        let mut loops = Vec::with_capacity(10);
        let root = kopen("/dev");
        for i in 0..10 {
            let name = format!("loop{}", i);
            debug!("[fs] create {} file", name);
            let loop0_dev_dentry = Arc::new(LoopDevDentry::new(
                Some(root.clone()),
                &name,
                root.super_block(),
            ));
            let loop0_dev_inode = Arc::new(LoopDevInode::new(root.super_block()));
            loop0_dev_dentry
                .into_dyn()
                .set_inode(loop0_dev_inode.clone());
            let loop0_file = Arc::new(LoopDevFile::new(
                loop0_dev_dentry.clone(),
                loop0_dev_inode,
                &FileFlags::O_RDWR,
                None,
            ));
            root.add_child(loop0_dev_dentry);
            loops.push(loop0_file);
        }

        Self { loops }
    }

    pub fn get(&self, id: usize) -> Option<Arc<LoopDevFile>> {
        if id < 10 {
            Some(self.loops[id].clone())
        } else {
            None
        }
    }

    pub fn get_free(&self) -> Option<usize> {
        for (i, dev) in self.loops.iter().enumerate() {
            if !dev.file.spin_lock().is_some() {
                return Some(i);
            }
        }
        None
    }
}

pub struct LoopControlFile {
    meta: FileMeta,
}

impl LoopControlFile {
    pub fn new(meta: FileMeta) -> Self {
        Self { meta }
    }
}

#[async_trait]
impl File for LoopControlFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!("readlink from tty");
    }
    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        Err(Errno::ENOSYS)
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        Err(Errno::ENOSYS)
    }

    async fn load_dir(&self) -> SysResult<()> {
        Err(Errno::ENOTDIR)
    }
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
    fn ioctl(&self, cmd: usize, _arg: usize) -> SyscallResult {
        const LOOP_CTL_GET_FREE: usize = 0x4c82;
        match cmd {
            LOOP_CTL_GET_FREE => {
                let id = LOOP_CONTROL.lock().get_free().ok_or(Errno::ENODEV)?;
                Ok(id as isize)
            }
            _ => {
                unimplemented!("Unsupported cmd: {}", cmd);
            }
        }
    }
    fn poll(&self, req: &PollEvent, _waker: Waker) -> PollEvent {
        let mut res = PollEvent::empty();
        if req.contains(PollEvent::POLLIN) {
            res |= PollEvent::POLLIN;
        }
        if req.contains(PollEvent::POLLOUT) {
            res |= PollEvent::POLLOUT;
        }
        res
    }
}
