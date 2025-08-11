use alloc::{boxed::Box, sync::Arc};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;

use super::{
    dentry::RamFsDentry,
    inode::{RamFsDirInode, RamFsFileInode},
};
use crate::{
    fs::vfs::{
        basic::file::{File, FileMeta},
        impls::ramfs::ramfs_write_guard,
    },
    include::{fs::FileFlags, io::PollEvent},
    syscall::{SysResult, SyscallResult},
};

pub struct RamFsFile {
    meta: FileMeta,
}

impl RamFsFile {
    pub fn new(
        dentry: Arc<RamFsDentry>,
        inode: Arc<RamFsFileInode>,
        file_flags: &FileFlags,
    ) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone(), file_flags),
        }
    }
}

#[async_trait]
impl File for RamFsFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let path = self.meta.dentry().path();
        let mut fs = ramfs_write_guard();
        let data = {
            if let Some(data) = fs.get_content(&path) {
                data
            } else {
                fs.add_file(path.clone());
                fs.get_content(&path).unwrap()
            }
        };

        debug!("[ramfs] open new file: {}", path);
        let inode = &self.meta.inode;
        let size = inode.size();
        debug!(
            "[ramfs] read offset: {}, size: {}, data: {:?}",
            offset, size, data
        );
        if offset >= size {
            return Ok(0); // EOF
        }

        let len = core::cmp::min(data.len() - offset, buf.len());
        buf[..len].copy_from_slice(&data[offset..offset + len]);
        Ok(len as isize)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        let path = self.meta.dentry().path();
        let mut fs = ramfs_write_guard();
        let data = {
            if let Some(data) = fs.get_content_mut(&path) {
                data
            } else {
                fs.add_file(path.clone());
                fs.get_content_mut(&path).unwrap()
            }
        };

        let inode = &self.meta.inode;
        let size = inode.size();
        debug!(
            "[ramfs] write offset: {}, size: {}, data: {:?}, buf: {:?}",
            offset, size, data, buf
        );
        if offset + buf.len() > size {
            inode.set_size(offset + buf.len());
        }
        data.resize(inode.size(), 0);

        data[offset..offset + buf.len()].copy_from_slice(&buf);
        Ok(buf.len() as isize)
    }
    async fn load_dir(&self) -> SysResult<()> {
        Err(Errno::ENOTDIR)
    }
    async fn delete_child(&self, name: &str) -> SysResult<()> {
        let parent = self.meta.dentry().parent().ok_or_else(|| {
            error!("[ramfs] delete_child called on root dentry");
            Errno::ENOENT
        })?;
        let mut parent_path = parent.path();
        parent_path.push_str(name);
        let mut fs = ramfs_write_guard();
        fs.remove_file(&parent_path);
        Ok(())
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
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

pub struct RamFsDir {
    meta: FileMeta,
}

impl RamFsDir {
    pub fn new(
        dentry: Arc<RamFsDentry>,
        inode: Arc<RamFsDirInode>,
        file_flags: &FileFlags,
    ) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone(), file_flags),
        }
    }
}

#[async_trait]
impl File for RamFsDir {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        Err(Errno::EISDIR)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        Err(Errno::EISDIR)
    }
    async fn load_dir(&self) -> SysResult<()> {
        Ok(())
    }
    /// Ram fs does not really has child or parent
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Ok(())
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
    }
    fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
        unreachable!("RamfsDir::poll not supported now");
    }
}
