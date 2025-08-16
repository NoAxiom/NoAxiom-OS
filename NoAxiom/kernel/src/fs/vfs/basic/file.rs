// ! **File** is the file system file instance opened in memory

use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{
    hash::{Hash, Hasher},
    intrinsics::unlikely,
    sync::atomic::{AtomicI32, AtomicUsize, Ordering},
    task::Waker,
};

use async_trait::async_trait;
use config::mm::PAGE_SIZE;
use downcast_rs::{impl_downcast, DowncastSync};

use super::{
    dentry::{self, Dentry},
    inode::{self, Inode},
};
use crate::{
    constant::fs::LEN_BEFORE_NAME,
    fs::{
        pagecache::{get_pagecache_rguard, get_pagecache_wguard, PageState},
        vfs::basic::{
            dentry::{DENTRY_FRONT, DENTRY_HERE},
            inode::InodeState,
        },
    },
    include::{
        fs::{FileFlags, LinuxDirent64, SeekFrom},
        io::PollEvent,
        result::Errno,
    },
    syscall::{SysResult, SyscallResult},
    utils::align_offset,
};

pub struct FileMeta {
    /// File flags, may be modified by multiple tasks
    flags: AtomicI32,
    /// The position of the file, may be modified by multiple tasks
    pub pos: AtomicUsize,
    /// Pointer to the Dentry
    dentry: Arc<dyn Dentry>,
    /// Pointer to the Inode
    pub inode: Arc<dyn Inode>,
}

impl FileMeta {
    pub fn new(dentry: Arc<dyn Dentry>, inode: Arc<dyn Inode>, flags: &FileFlags) -> Self {
        Self {
            flags: AtomicI32::new(flags.bits()),
            pos: AtomicUsize::new(0),
            dentry,
            inode,
        }
    }
    #[allow(unused)]
    pub fn empty() -> Self {
        let dentry = Arc::new(dentry::EmptyDentry::new("empty-file"));
        let inode = Arc::new(inode::EmptyInode::new());
        Self::new(dentry, inode, &FileFlags::empty())
    }
    pub fn dentry(&self) -> Arc<dyn Dentry> {
        self.dentry.clone()
    }
    pub fn readable(&self) -> bool {
        let flags =
            FileFlags::from_bits(self.flags.load(Ordering::SeqCst)).unwrap_or(FileFlags::empty());
        !flags.contains(FileFlags::O_WRONLY) || flags.contains(FileFlags::O_RDWR)
    }
    pub fn writable(&self) -> bool {
        let flags =
            FileFlags::from_bits(self.flags.load(Ordering::SeqCst)).unwrap_or(FileFlags::empty());
        flags.contains(FileFlags::O_WRONLY) || flags.contains(FileFlags::O_RDWR)
    }
    pub fn set_flags(&self, flags: FileFlags) {
        self.flags.store(flags.bits(), Ordering::SeqCst);
    }
    pub fn set_nonblock(&self) {
        let mut flags =
            FileFlags::from_bits(self.flags.load(Ordering::SeqCst)).unwrap_or(FileFlags::empty());
        flags |= FileFlags::O_NONBLOCK;
        self.set_flags(flags);
    }
    pub fn is_nonblocking(&self) -> bool {
        let flags =
            FileFlags::from_bits(self.flags.load(Ordering::SeqCst)).unwrap_or(FileFlags::empty());
        flags.contains(FileFlags::O_NONBLOCK)
    }
}

#[async_trait]
pub trait File: Send + Sync + DowncastSync {
    /// Get the size of file
    fn size(&self) -> usize {
        self.meta().inode.size()
    }
    fn page_cache(&self) -> Option<()> {
        self.meta().inode.page_cache()
    }
    /// Get the dentry of the file
    fn dentry(&self) -> Arc<dyn Dentry> {
        self.meta().dentry.clone()
    }
    /// Can be interrupted by signal? Default is false.
    fn is_interruptable(&self) -> bool {
        false
    }
    /// Get the meta of the file
    fn meta(&self) -> &FileMeta;
    /// Read data from file at `offset` to `buf`, not for kernel other modules
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult;
    /// Readlink data from file at `offset` to `buf`
    async fn base_readlink(&self, buf: &mut [u8]) -> SyscallResult;
    /// Write data to file at `offset` from `buf`, not for kernel other modules
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult;
    /// Load directory into memory, must be called before read/write explicitly,
    /// only for directories
    async fn load_dir(&self) -> Result<(), Errno>;
    /// Delete dentry, only for directories
    async fn delete_child(&self, name: &str) -> Result<(), Errno>;
    /// IOCTL command
    #[allow(unused)]
    fn ioctl(&self, cmd: usize, arg: usize) -> SyscallResult;
    fn poll(&self, req: &PollEvent, waker: Waker) -> PollEvent;
    // {
    //     let mut res = PollEvent::empty();
    //     if req.contains(PollEvent::POLLIN) {
    //         res |= PollEvent::POLLIN;
    //     }
    //     if req.contains(PollEvent::POLLOUT) {
    //         res |= PollEvent::POLLOUT;
    //     }
    //     res
    // }
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

    pub async fn read_all(self: &Arc<dyn File>) -> SysResult<Vec<u8>> {
        let len = self.meta().inode.size();
        let mut buf = vec![0; len];
        self.read_at(0, &mut buf).await?;
        Ok(buf)
    }

    /// **FOR KERNEL!**  
    /// read the file at `offset` and fill the `buf` as much as possible
    /// by using the page cache  
    ///
    /// if missing, using the `base_read` to fill the
    /// page cache
    ///
    /// return the exact num of bytes read
    pub async fn read_at(self: &Arc<dyn File>, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let page_cache = self.page_cache();
        if page_cache.is_none() {
            assert_no_lock!();
            return self.base_read(offset, buf).await;
        }
        let size = self.size();
        if offset > size {
            warn!(
                "[read_at] {} offset {offset} > size {size}",
                self.meta().dentry.name()
            );
            return Ok(0);
        }

        let mut current_offset = offset;
        let mut buf = buf;

        // if the last page is not full,
        // if the buf is run out
        loop {
            let (offset_align, offset_in) = align_offset(current_offset, PAGE_SIZE);
            // maybe it is the last page, the exact len is equal to `size` -
            // `current_offset` or the rest of the page
            let page_len = (PAGE_SIZE - offset_in).min(size - current_offset);
            // maybe the buf is not enough
            let len = buf.len().min(page_len);

            let r_guard = get_pagecache_rguard();
            if let Some(page) = r_guard.get_page(&self.clone(), offset_align) {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        page.as_mut_bytes_array().as_ptr().add(offset_in),
                        buf.as_mut_ptr(),
                        len,
                    );
                }
                // debug!(
                //     "[read_at] {} at {offset_align}, file_ino: {}, file_size: {}, content:
                // {:?}",     self.name(),
                //     self.meta().inode.id(),
                //     size,
                //     &page.as_mut_bytes_array()[..10],
                // );

                buf = &mut buf[len..];
                current_offset += len;

                if buf.is_empty() || current_offset == size {
                    break;
                }

                continue;
            }
            drop(r_guard);
            let page_state = if self.inode().state() == InodeState::Deleted {
                PageState::Deleted
            } else {
                PageState::Shared
            };
            let mut w_guard = get_pagecache_wguard();
            let page = w_guard.alloc(page_state);
            drop(w_guard);
            self.base_read(offset_align, page.as_mut_bytes_array())
                .await?;
            let mut w_guard = get_pagecache_wguard();
            w_guard.fill_page(&self.clone(), offset_align, page);
        }

        Ok((current_offset - offset) as isize)
    }

    pub async fn read(self: &Arc<dyn File>, buf: &mut [u8]) -> SyscallResult {
        let offset = self.meta().pos.load(Ordering::Acquire);
        let len = self.read_at(offset, buf).await?;
        self.meta().pos.fetch_add(len as usize, Ordering::Release);
        Ok(len)
    }

    /// **FOR KERNEL!**  
    /// write the `buf` to the file at `offset` as much as possible by using
    /// the page cache
    ///
    /// if missing, using the `base_read` to fill the
    /// page cache
    ///
    /// return the exact num of bytes write
    pub async fn write_at(self: &Arc<dyn File>, offset: usize, buf: &[u8]) -> SyscallResult {
        if buf.is_empty() {
            return Ok(0);
        }

        let page_cache = self.page_cache();
        if page_cache.is_none() {
            assert_no_lock!();
            return self.base_write(offset, buf).await;
        }
        let size = self.size();

        let mut current_offset = offset;
        let mut buf = buf;
        let ret = buf.len();

        let end_size = offset + ret;
        if end_size > size {
            trace!(
                "[write_at] {} expand size to {end_size}",
                self.meta().dentry.name(),
            );
            self.inode().set_size(end_size);
        }

        let mut w_guard = get_pagecache_wguard();

        // if the last page is not full,
        // if the buf is run out
        loop {
            let (offset_align, offset_in) = align_offset(current_offset, PAGE_SIZE);
            // maybe it is the last page, the exact len is equal to `size` -
            // `current_offset` mention that write can expand the file, so the
            // file size is not useful
            let page_len = PAGE_SIZE - offset_in;
            // maybe the buf is not enough
            let len = buf.len().min(page_len);
            if let Some(page) = w_guard.get_page_mut(&self.clone(), offset_align) {
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        buf.as_ptr(),
                        page.as_mut_bytes_array().as_mut_ptr().add(offset_in),
                        len,
                    );
                }
                page.mark_dirty();

                buf = &buf[len..];
                current_offset += len;

                if buf.is_empty() {
                    break;
                }

                continue;
            }

            let page_state = if self.inode().state() == InodeState::Deleted {
                PageState::Deleted
            } else {
                PageState::Shared
            };
            let page = w_guard.alloc(page_state);
            self.base_read(offset_align, page.as_mut_bytes_array())
                .await?;
            w_guard.fill_page(&self.clone(), offset_align, page);
        }

        Ok(ret as isize)
    }

    pub async fn write(self: &Arc<dyn File>, buf: &[u8]) -> SyscallResult {
        let flags = self.flags();
        let offset = if unlikely(flags.contains(FileFlags::O_APPEND)) {
            // For O_APPEND, always write at the end of the file
            self.size()
        } else {
            self.meta().pos.load(Ordering::Acquire)
        };

        let len = self.write_at(offset, buf).await?;

        // Update file position to the end of the written data
        self.meta()
            .pos
            .store(offset + len as usize, Ordering::Release);
        Ok(len)
    }
    #[inline(always)]
    pub fn name(&self) -> String {
        self.dentry().name().to_string()
    }
    #[inline(always)]
    pub fn path(&self) -> String {
        self.dentry().path().to_string()
    }
    #[inline(always)]
    pub fn pos(&self) -> usize {
        self.meta().pos.load(Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn flags(&self) -> FileFlags {
        FileFlags::from_bits(self.meta().flags.load(Ordering::SeqCst)).unwrap_or(FileFlags::empty())
    }
    #[inline(always)]
    pub fn inode(&self) -> Arc<dyn Inode> {
        self.meta().inode.clone()
    }

    pub fn truncate_pagecache(self: &Arc<dyn File>, length: usize) {
        let mut w_guard = get_pagecache_wguard();
        w_guard.truncate(&self.clone(), length);
    }

    /// Reference: Phoenix  
    /// Read directory entries from the directory file.
    pub async fn read_dir(&self, buf: &mut [u8]) -> SyscallResult {
        self.load_dir().await?;
        if buf.len() < core::mem::size_of::<LinuxDirent64>() {
            return Err(Errno::EINVAL);
        }

        let buf_len = buf.len();
        let mut writen_len = 0;
        let mut buf_it = buf;
        let dentry = self.dentry();
        let mut children = dentry.children().clone();
        children.insert(".".to_string(), DENTRY_HERE.clone());
        children.insert("..".to_string(), DENTRY_FRONT.clone());
        let offset = self.meta().pos.load(Ordering::Relaxed);
        for (name, dentry) in children.iter().skip(offset) {
            if dentry.is_negative() {
                self.seek(SeekFrom::Current(1))?;
                continue;
            }
            // align to 8 bytes
            let c_name_len = name.len() + 1;
            let rec_len = (LEN_BEFORE_NAME + c_name_len + 7) & !0x7;
            let inode = dentry.inode()?;
            let linux_dirent = LinuxDirent64::new(
                inode.id() as u64,
                offset as u64,
                rec_len as u16,
                (inode.file_type().bits() >> 12) as u8,
            );

            debug!(
                "[sys_getdents64] dentry {:?}, linux dirent {linux_dirent:?}",
                dentry.path()
            );
            if writen_len + rec_len > buf_len {
                break;
            }

            self.seek(SeekFrom::Current(1))?;
            let ptr = buf_it.as_mut_ptr() as *mut LinuxDirent64;
            unsafe {
                ptr.copy_from_nonoverlapping(&linux_dirent, 1);
            }
            buf_it[LEN_BEFORE_NAME..LEN_BEFORE_NAME + c_name_len - 1]
                .copy_from_slice(name.as_bytes());
            buf_it[LEN_BEFORE_NAME + c_name_len - 1] = b'\0';
            buf_it = &mut buf_it[rec_len..];
            writen_len += rec_len;
        }
        Ok(writen_len as isize)
    }
}

impl Hash for dyn File {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.meta().inode.id().hash(state);
    }
}
impl PartialEq for dyn File {
    fn eq(&self, other: &Self) -> bool {
        self.meta().inode.id() == other.meta().inode.id()
    }
}
impl Eq for dyn File {}

pub struct EmptyFile {
    meta: FileMeta,
}

impl EmptyFile {
    #[allow(unused)]
    pub fn new() -> Self {
        let dentry = Arc::new(dentry::EmptyDentry::new("empty-file"));
        let inode = Arc::new(inode::EmptyInode::new());
        Self {
            meta: FileMeta::new(dentry, inode, &FileFlags::empty()),
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
    fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
        unreachable!()
    }
}
