// ! **File** is the file system file instance opened in memory

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use downcast_rs::{impl_downcast, DowncastSync};
use spin::Mutex;

use super::{
    dentry::{self, Dentry},
    inode::{self, Inode},
};
use crate::{
    constant::fs::LEN_BEFORE_NAME,
    include::{
        fs::{FileFlags, LinuxDirent64, SeekFrom},
        io::PollEvent,
        result::Errno,
    },
    syscall::{SysResult, SyscallResult},
};

pub struct FileMeta {
    /// File flags, may be modified by multiple tasks
    flags: Mutex<FileFlags>,
    /// The position of the file, may be modified by multiple tasks
    pub pos: AtomicUsize,
    /// Pointer to the Dentry
    dentry: Arc<dyn Dentry>,
    /// Pointer to the Inode
    pub inode: Arc<dyn Inode>,
}

impl FileMeta {
    pub fn new(dentry: Arc<dyn Dentry>, inode: Arc<dyn Inode>) -> Self {
        Self {
            flags: Mutex::new(FileFlags::empty()),
            pos: AtomicUsize::new(0),
            dentry,
            inode,
        }
    }
    #[allow(unused)]
    pub fn empty() -> Self {
        let dentry = Arc::new(dentry::EmptyDentry::new());
        let inode = Arc::new(inode::EmptyInode::new());
        Self::new(dentry, inode)
    }
    pub fn dentry(&self) -> Arc<dyn Dentry> {
        self.dentry.clone()
    }
    pub fn readable(&self) -> bool {
        let flags = self.flags.lock();
        !flags.contains(FileFlags::O_WRONLY) || flags.contains(FileFlags::O_RDWR)
    }
    pub fn writable(&self) -> bool {
        let flags = self.flags.lock();
        flags.contains(FileFlags::O_WRONLY) || flags.contains(FileFlags::O_RDWR)
    }
    pub fn set_flags(&self, flags: FileFlags) {
        *self.flags.lock() = flags;
    }
}

#[async_trait]
pub trait File: Send + Sync + DowncastSync {
    /// Get the size of file
    fn size(&self) -> usize {
        self.meta().inode.size()
    }
    /// Get the dentry of the file
    fn dentry(&self) -> Arc<dyn Dentry> {
        self.meta().dentry.clone()
    }
    /// Get the meta of the file
    fn meta(&self) -> &FileMeta;
    /// Read data from file at `offset` to `buf`
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult;
    /// Readlink data from file at `offset` to `buf`
    async fn base_readlink(&self, buf: &mut [u8]) -> SyscallResult;
    /// Write data to file at `offset` from `buf`
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult;
    /// Load directory into memory, must be called before read/write explicitly,
    /// only for directories
    async fn load_dir(&self) -> Result<(), Errno>;
    /// Delete dentry, only for directories
    async fn delete_child(&self, name: &str) -> Result<(), Errno>;
    /// IOCTL command
    #[allow(unused)]
    fn ioctl(&self, cmd: usize, arg: usize) -> SyscallResult;
    fn poll(&self, req: &PollEvent) -> PollEvent {
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

impl_downcast!(sync File);

impl dyn File {
    /// Called when the VFS needs to move the file position index.
    ///
    /// Return the result offset.
    pub fn seek(&self, pos: SeekFrom) -> SyscallResult {
        let mut res_pos = self.meta().pos.load(Ordering::Acquire);
        match pos {
            SeekFrom::Current(offset) => {
                if offset < 0 {
                    if res_pos as i64 - offset.abs() < 0 {
                        return Err(Errno::EINVAL);
                    }
                    res_pos -= offset.abs() as usize;
                } else {
                    res_pos += offset as usize;
                }
            }
            SeekFrom::Start(offset) => {
                res_pos = offset as usize;
            }
            SeekFrom::End(offset) => {
                let size = self.size();
                if offset < 0 {
                    res_pos = size - offset.abs() as usize;
                } else {
                    res_pos = size + offset as usize;
                }
            }
        }
        self.meta().pos.store(res_pos, Ordering::Release);
        Ok(res_pos as isize)
    }

    pub async fn read_all(&self) -> SysResult<Vec<u8>> {
        let len = self.meta().inode.size();
        let mut buf = vec![0; len];
        self.base_read(0, &mut buf).await?;
        Ok(buf)
    }
    pub async fn read(&self, buf: &mut [u8]) -> SyscallResult {
        let offset = self.meta().pos.load(Ordering::Acquire);
        let len = self.base_read(offset, buf).await?;
        self.meta().pos.fetch_add(len as usize, Ordering::Release);
        Ok(len)
    }
    pub async fn write(&self, buf: &[u8]) -> SyscallResult {
        let offset = self.meta().pos.load(Ordering::Acquire);
        let len = self.base_write(offset, buf).await?;
        self.meta().pos.fetch_add(len as usize, Ordering::Release);
        Ok(len)
    }
    pub fn name(&self) -> String {
        self.dentry().name()
    }
    pub fn flags(&self) -> spin::MutexGuard<'_, FileFlags> {
        self.meta().flags.lock()
    }
    pub fn set_flags(&self, flags: FileFlags) {
        *self.meta().flags.lock() = flags;
    }
    pub fn inode(&self) -> Arc<dyn Inode> {
        self.meta().inode.clone()
    }

    /// Reference: Phoenix  
    /// Read directory entries from the directory file.
    pub async fn read_dir(&self, buf: &mut [u8]) -> SyscallResult {
        self.load_dir().await?;

        let buf_len = buf.len();
        let mut writen_len = 0;
        let mut buf_it = buf;
        let dentry = self.dentry();
        let children = dentry.children();
        let offset = self.meta().pos.load(Ordering::Relaxed);
        for dentry in children.values().skip(offset) {
            if dentry.is_negative() {
                self.seek(SeekFrom::Current(1))?;
                continue;
            }
            // align to 8 bytes
            let c_name_len = dentry.name().len() + 1;
            let rec_len = (LEN_BEFORE_NAME + c_name_len + 7) & !0x7;
            let inode = dentry.inode()?;
            let linux_dirent = LinuxDirent64::new(
                inode.id() as u64,
                offset as u64,
                rec_len as u16,
                inode.file_type().bits() as u8,
            );

            trace!("[sys_getdents64] linux dirent {linux_dirent:?}");
            if writen_len + rec_len > buf_len {
                break;
            }

            self.seek(SeekFrom::Current(1))?;
            let ptr = buf_it.as_mut_ptr() as *mut LinuxDirent64;
            unsafe {
                ptr.copy_from_nonoverlapping(&linux_dirent, 1);
            }
            buf_it[LEN_BEFORE_NAME..LEN_BEFORE_NAME + c_name_len - 1]
                .copy_from_slice(dentry.name().as_bytes());
            buf_it[LEN_BEFORE_NAME + c_name_len - 1] = b'\0';
            buf_it = &mut buf_it[rec_len..];
            writen_len += rec_len;
        }
        Ok(writen_len as isize)
    }
}

pub struct EmptyFile {
    meta: FileMeta,
}

impl EmptyFile {
    #[allow(unused)]
    pub fn new() -> Self {
        let dentry = Arc::new(dentry::EmptyDentry::new());
        let inode = Arc::new(inode::EmptyInode::new());
        Self {
            meta: FileMeta::new(dentry, inode),
        }
    }
}

#[async_trait]
impl File for EmptyFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        unreachable!()
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        unreachable!()
    }
    async fn delete_child(&self, _name: &str) -> Result<(), Errno> {
        unreachable!()
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        unreachable!()
    }
}
