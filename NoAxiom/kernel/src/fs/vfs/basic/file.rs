// ! **File** is the file system file instance opened in memory

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::sync::atomic::AtomicUsize;

use async_trait::async_trait;
use fatfs::SeekFrom;
type Mutex<T> = ksync::mutex::SpinLock<T>;

use core::sync::atomic::Ordering;

use super::{
    dentry::{self, Dentry},
    inode::{self, Inode},
};
use crate::{
    constant::fs::LEN_BEFORE_NAME,
    include::{
        fs::{FileFlags, LinuxDirent64},
        result::Errno,
    },
    syscall::SyscallResult,
};

pub struct FileMeta {
    /// File flags, may be modified by multiple tasks
    flags: Mutex<FileFlags>,
    /// The position of the file, may be modified by multiple tasks
    pos: AtomicUsize,
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
}

#[async_trait]
pub trait File: Send + Sync {
    /// Get the size of file
    fn size(&self) -> usize {
        self.meta().inode.size()
    }
    /// Get the dentry of the file
    fn dentry(&self) -> Arc<dyn Dentry> {
        self.meta().dentry.clone()
    }
    /// Is STD_IN or STD_OUT or STD_ERR
    fn is_stdio(&self) -> bool {
        false
    }
    /// Get the meta of the file
    fn meta(&self) -> &FileMeta;
    /// Read data from file at `offset` to `buf`
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult;
    /// Write data to file at `offset` from `buf`
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult;
    /// Load directory into memory, must be called before read/write explicitly,
    /// only for directories
    async fn load_dir(&self) -> Result<(), Errno>;
    /// Delete dentry, only for directories
    async fn delete_child(&self, name: &str) -> Result<(), Errno>;
}

impl dyn File {
    fn pos(&self) -> usize {
        self.meta().pos.load(Ordering::Relaxed)
    }

    fn set_pos(&self, pos: usize) {
        self.meta().pos.store(pos, Ordering::Relaxed);
    }

    /// Called when the VFS needs to move the file position index.
    ///
    /// Return the result offset.
    fn seek(&self, pos: SeekFrom) -> SyscallResult {
        let mut res_pos = self.pos();
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
        self.set_pos(res_pos);
        Ok(res_pos as isize)
    }

    pub async fn read_all(&self) -> Result<Vec<u8>, Errno> {
        let len = self.meta().inode.size();
        let mut buf = vec![0; len];
        self.base_read(0, &mut buf).await?;
        Ok(buf)
    }
    pub async fn read(&self, buf: &mut [u8]) -> SyscallResult {
        let offset = self.pos();
        let file_size = self.meta().inode.size();
        if offset + buf.len() > file_size {
            warn!("read beyond file size, truncate the buffer!!");
            let len = file_size - offset;
            let mut new_buf = vec![0; len];
            let res_len = self.base_read(offset, &mut new_buf).await?;
            buf[..len as usize].copy_from_slice(&new_buf);
            Ok(res_len)
        } else {
            let len = self.base_read(offset, buf).await?;
            self.meta().pos.fetch_add(len as usize, Ordering::Relaxed);
            Ok(len)
        }
    }
    pub async fn write(&self, buf: &mut [u8]) -> SyscallResult {
        let offset = self.pos();
        let len = self.base_write(offset, buf).await?;
        self.meta().pos.fetch_add(len as usize, Ordering::Relaxed);
        Ok(len)
    }
    pub fn name(&self) -> String {
        self.dentry().name()
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
        let children = self.dentry().children();
        let offset = self.pos();
        for dentry in children.values().skip(offset) {
            if dentry.is_negetive() {
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

            debug!("[sys_getdents64] linux dirent {linux_dirent:?}");
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
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        unreachable!()
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        unreachable!()
    }
    async fn delete_child(&self, _name: &str) -> Result<(), Errno> {
        unreachable!()
    }
}
