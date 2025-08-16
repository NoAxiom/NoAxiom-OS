use alloc::{boxed::Box, string::ToString, sync::Arc, vec::Vec};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;

use crate::{
    cpu::current_task,
    fs::vfs::{
        basic::{
            dentry::Dentry,
            file::{File, FileMeta},
            inode::Inode,
        },
        impls::proc::fd::{dentry::FdDentry, inode::FdFileInode},
    },
    include::io::PollEvent,
    syscall::{SysResult, SyscallResult},
    task::manager::TASK_MANAGER,
};

pub struct FdDir {
    meta: FileMeta,
}

impl FdDir {
    pub fn new(meta: FileMeta) -> Self {
        Self { meta }
    }
}

enum StatTid {
    SelfTid,
    Tid(usize),
}

fn resolve_path_tid(path: &str) -> StatTid {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 3 || parts[1] != "proc" {
        panic!("Invalid proc path: {}", path);
    }
    if parts[2] == "self" {
        StatTid::SelfTid
    } else {
        StatTid::Tid(parts[2].parse().expect("Failed to parse tid from path"))
    }
}

#[async_trait]
impl File for FdDir {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        Err(Errno::ENOSYS)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        unreachable!("write to FdFile");
    }
    async fn load_dir(&self) -> SysResult<()> {
        let path = self.meta.dentry().path();
        let tid = resolve_path_tid(&path);
        let task = match tid {
            StatTid::SelfTid => current_task().unwrap(),
            StatTid::Tid(x) => &TASK_MANAGER.get(x).ok_or_else(|| {
                error!("[FdFile::load_dir] Failed to get task, return EINVAL");
                Errno::EINVAL
            })?,
        };

        let superblock = self.meta.dentry().super_block();
        self.dentry().children().clear();
        let fd_table = task.fd_table();
        for (fd, entry) in fd_table.table.iter().enumerate() {
            if let Some(entry) = entry {
                let file = &entry.file;
                let son: Arc<dyn Dentry> = Arc::new(FdDentry::new(
                    Some(self.meta.dentry()),
                    &fd.to_string(),
                    superblock.clone(),
                ));
                let inode: Arc<dyn Inode> = Arc::new(FdFileInode::new(superblock.clone()));
                inode.set_symlink(file.path());
                son.set_inode(inode);
                self.dentry().add_child(son);
            }
        }
        drop(fd_table);

        Ok(())
    }
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
    }
    fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
        unreachable!("FdFile::poll not supported now");
    }
}

pub struct FdFile {
    meta: FileMeta,
    tid: StatTid,
    fd: usize,
}

#[async_trait]
impl File for FdFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let task = match self.tid {
            StatTid::SelfTid => current_task().unwrap(),
            StatTid::Tid(x) => &TASK_MANAGER.get(x).ok_or_else(|| {
                error!("[FdFile::load_dir] Failed to get task, return EINVAL");
                Errno::EINVAL
            })?,
        };
        let file = task.fd_table().get(self.fd).ok_or(Errno::EBADF)?;
        file.read_at(offset, buf).await
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        unreachable!("write to FdFile");
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
        unreachable!("FdFile::poll not supported now");
    }
}
