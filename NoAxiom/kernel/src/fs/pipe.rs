//! Pipe
//!
//! 1. 0 read ends && 0 write ends: ok
//!
//! 2. x read ends && 0 write ends: reading the remaining data, if try to read
//!    again, get EOF
//!
//! 3. 0 read ends && x write ends: write end process get SIGPIPE
//!
//! 4. x read ends && x write ends:
//!   - read until pipe empty or buf end
//!   - write until pipe full or buf end

use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use async_trait::async_trait;
use ksync::mutex::SpinLock;

use super::vfs::{
    basic::{
        dentry::{Dentry, DentryMeta},
        file::{File, FileMeta},
        inode::InodeMeta,
    },
    root_dentry,
};
use crate::{
    config::fs::PIPE_BUF_SIZE,
    fs::vfs::basic::inode::Inode,
    include::{
        fs::{FileFlags, InodeMode, Stat},
        io::PollEvent,
        result::Errno,
    },
    syscall::{SysResult, SyscallResult},
    utils::random,
};

#[derive(PartialEq)]
enum PipeBufferStatus {
    Empty,
    Full,
    Normal,
}

/// Ring buffer that max size is PIPE_BUF_SIZE
struct PipeBuffer {
    data: [u8; PIPE_BUF_SIZE],
    head: usize,
    tail: usize,
    status: PipeBufferStatus,
    read_wakers: Vec<Waker>,
    write_wakers: Vec<Waker>,
    /// used to count the number of read and write ends
    read_end: Option<Weak<PipeFile>>,
    write_end: Option<Weak<PipeFile>>,
}

impl PipeBuffer {
    fn new() -> Self {
        Self {
            data: [0; PIPE_BUF_SIZE],
            head: 0,
            tail: 0,
            status: PipeBufferStatus::Empty,
            read_wakers: Vec::new(),
            write_wakers: Vec::new(),
            read_end: None,
            write_end: None,
        }
    }
    fn add_read_event(&mut self, waker: Waker) {
        self.read_wakers.push(waker);
    }
    fn add_write_event(&mut self, waker: Waker) {
        self.write_wakers.push(waker);
    }
    fn set_read_end(&mut self, read_end: Weak<PipeFile>) {
        self.read_end = Some(read_end);
    }
    fn set_write_end(&mut self, write_end: Weak<PipeFile>) {
        self.write_end = Some(write_end);
    }
    fn has_writend(&self) -> bool {
        self.write_end.is_some()
    }
    fn has_readend(&self) -> bool {
        self.read_end.is_some()
    }
    fn read_available(&self) -> usize {
        match self.status {
            PipeBufferStatus::Empty => 0,
            PipeBufferStatus::Full => PIPE_BUF_SIZE,
            PipeBufferStatus::Normal => {
                if self.head <= self.tail {
                    self.tail - self.head
                } else {
                    PIPE_BUF_SIZE - self.head + self.tail
                }
            }
        }
    }
    fn write_available(&self) -> usize {
        match self.status {
            PipeBufferStatus::Empty => PIPE_BUF_SIZE,
            PipeBufferStatus::Full => 0,
            PipeBufferStatus::Normal => {
                if self.head <= self.tail {
                    PIPE_BUF_SIZE - self.tail + self.head
                } else {
                    self.head - self.tail
                }
            }
        }
    }
    fn notify_read_waker(&mut self) {
        if let Some(waker) = self.read_wakers.pop() {
            waker.wake();
        }
    }
    fn notify_write_waker(&mut self) {
        if let Some(waker) = self.write_wakers.pop() {
            waker.wake();
        }
    }
    /// Read `len` bytes as much as possible from the buffer, make sure buffer's
    /// size >= len, return the number of bytes read
    fn read(&mut self, buf: &mut [u8]) -> usize {
        trace!("[PipeBuffer] read buf");
        let len = buf.len();
        let res = match self.status {
            PipeBufferStatus::Empty => 0,
            _ => {
                if self.head < self.tail {
                    let res = core::cmp::min(len, self.tail - self.head);
                    buf[..res].copy_from_slice(&self.data[self.head..self.head + res]);
                    res
                } else {
                    // maybe full
                    let res = core::cmp::min(len, PIPE_BUF_SIZE - self.head + self.tail);
                    if res <= PIPE_BUF_SIZE - self.head {
                        buf[..res].copy_from_slice(&self.data[self.head..self.head + res]);
                    } else {
                        buf[..PIPE_BUF_SIZE - self.head].copy_from_slice(&self.data[self.head..]);
                        buf[PIPE_BUF_SIZE - self.head..res]
                            .copy_from_slice(&self.data[..self.head + res - PIPE_BUF_SIZE]);
                    }
                    res
                }
            }
        };
        self.head = (self.head + res) % PIPE_BUF_SIZE;
        if self.head == self.tail {
            self.status = PipeBufferStatus::Empty;
        } else {
            self.status = PipeBufferStatus::Normal;
        }
        res
    }
    /// Write `buf` as much as possible to the buffer
    fn write(&mut self, buf: &[u8]) -> usize {
        trace!(
            "[PipeBuffer] write buf as string: {}",
            alloc::string::String::from_utf8_lossy(buf)
        );
        let len = buf.len();
        let res = match self.status {
            PipeBufferStatus::Full => 0,
            _ => {
                if self.head <= self.tail {
                    // maybe empty
                    trace!("[PipeBuffer] write maybe empty");
                    let res = core::cmp::min(len, self.head + PIPE_BUF_SIZE - self.tail);
                    if res <= PIPE_BUF_SIZE - self.tail {
                        self.data[self.tail..self.tail + res].copy_from_slice(&buf[..res]);
                    } else {
                        self.data[self.tail..].copy_from_slice(&buf[..PIPE_BUF_SIZE - self.tail]);
                        self.data[..self.tail + res - PIPE_BUF_SIZE]
                            .copy_from_slice(&buf[PIPE_BUF_SIZE - self.tail..res]);
                    }
                    res
                } else {
                    debug!("[PipeBuffer] write normal");
                    let res = core::cmp::min(len, self.head - self.tail);
                    self.data[self.tail..self.tail + res].copy_from_slice(&buf[..res]);
                    res
                }
            }
        };
        self.tail = (self.tail + res) % PIPE_BUF_SIZE;
        if self.head == self.tail {
            self.status = PipeBufferStatus::Full;
        } else {
            self.status = PipeBufferStatus::Normal;
        }
        res
    }
}

pub struct PipeDentry {
    meta: DentryMeta,
}

impl PipeDentry {
    /// we mount all the pipes to the root dentry
    pub fn new(name: &str) -> Arc<Self> {
        let parent = root_dentry();
        let super_block = parent.super_block();
        let pipe_dentry = Arc::new(Self {
            meta: DentryMeta::new(Some(parent.clone()), name, super_block),
        });
        debug!("[PipeDentry] create pipe dentry: {}", pipe_dentry.name());
        parent.add_child_directly(pipe_dentry.clone());
        pipe_dentry
    }
}

#[async_trait]
impl Dentry for PipeDentry {
    fn meta(&self) -> &DentryMeta {
        &self.meta
    }

    fn from_name(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        unreachable!("pipe dentry should not have child");
    }

    fn open(self: Arc<Self>) -> SysResult<Arc<dyn File>> {
        unreachable!("pipe dentry should not open");
    }

    async fn create(self: Arc<Self>, _name: &str, _mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        unreachable!("pipe dentry should not create child");
    }
}

pub struct PipeInode {
    meta: InodeMeta,
}

impl PipeInode {
    pub fn new() -> Self {
        let parent = root_dentry();
        let super_block = parent.super_block();
        Self {
            meta: InodeMeta::new(super_block, InodeMode::FIFO, PIPE_BUF_SIZE, false),
        }
    }
}

impl Inode for PipeInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn stat(&self) -> SysResult<Stat> {
        let inner = self.meta.inner.lock();
        Ok(Stat {
            st_dev: 0,
            st_ino: self.meta.id as u64,
            st_mode: self.meta.inode_mode.bits(),
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: inner.size as u64,
            st_blksize: 0,
            __pad2: 0,
            st_blocks: 0 as u64,
            st_atime_sec: inner.atime_sec as u64,
            st_atime_nsec: inner.atime_nsec as u64,
            st_mtime_sec: inner.mtime_sec as u64,
            st_mtime_nsec: inner.mtime_nsec as u64,
            st_ctime_sec: inner.ctime_sec as u64,
            st_ctime_nsec: inner.ctime_nsec as u64,
            unused: 0,
        })
    }
}

pub struct PipeFile {
    buffer: Arc<SpinLock<PipeBuffer>>,
    meta: FileMeta,
}

impl PipeFile {
    pub fn into_dyn(self: Arc<Self>) -> Arc<dyn File> {
        self.clone()
    }
    fn new_read_end(buffer: Arc<SpinLock<PipeBuffer>>, name: &str) -> Arc<Self> {
        let name = format!("{}-read", name);
        let dentry = PipeDentry::new(&name);
        let inode = Arc::new(PipeInode::new());
        dentry.set_inode(inode.clone());
        let meta = FileMeta::new(dentry, inode);
        let res = Arc::new(Self { buffer, meta });
        res.clone().into_dyn().set_flags(FileFlags::O_RDONLY);
        res
    }
    fn new_write_end(buffer: Arc<SpinLock<PipeBuffer>>, name: &str) -> Arc<Self> {
        let name = format!("{}-write", name);
        let dentry = PipeDentry::new(&name);
        let inode = Arc::new(PipeInode::new());
        dentry.set_inode(inode.clone());
        let meta = FileMeta::new(dentry, inode);
        let res = Arc::new(Self { buffer, meta });
        res.clone().into_dyn().set_flags(FileFlags::O_WRONLY);
        res
    }
    fn is_read_end(&self) -> bool {
        self.meta.readable()
    }
    fn is_write_end(&self) -> bool {
        self.meta.writable()
    }
    /// Create a new pipe, return (read end, write end)
    pub fn new_pipe() -> (Arc<Self>, Arc<Self>) {
        let buffer = Arc::new(SpinLock::new(PipeBuffer::new()));
        let name = format!("pipe-{}", random());
        let read_end = Self::new_read_end(buffer.clone(), &name);
        let write_end = Self::new_write_end(buffer.clone(), &name);
        let read_end_weak = Arc::downgrade(&read_end);
        let write_end_weak = Arc::downgrade(&write_end);
        buffer.lock().set_read_end(read_end_weak);
        buffer.lock().set_write_end(write_end_weak);
        (read_end, write_end)
    }
}

#[async_trait]
impl File for PipeFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, _offset: usize, buf: &mut [u8]) -> SyscallResult {
        assert!(self.is_read_end());
        debug!("[pipe] {} read, {}", self.meta.dentry().name(), buf.len());
        PipeReadFuture::new(self.buffer.clone()).await?;
        let mut buffer = self.buffer.lock();
        let ret = buffer.read(buf);
        debug!(
            "[pipe] {} read buf as string: {}",
            self.meta.dentry().name(),
            alloc::string::String::from_utf8_lossy(buf)
        );
        buffer.notify_write_waker();
        Ok(ret as isize)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, buf: &[u8]) -> SyscallResult {
        assert!(self.is_write_end());
        debug!("[pipe] {} write, {}", self.meta.dentry().name(), buf.len());
        PipeWriteFuture::new(self.buffer.clone()).await?;
        let mut buffer = self.buffer.lock();
        let ret = buffer.write(buf);
        debug!(
            "[pipe] {} write buf as string: {}",
            self.meta.dentry().name(),
            alloc::string::String::from_utf8_lossy(buf)
        );
        buffer.notify_read_waker();
        Ok(ret as isize)
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        Err(Errno::ENOTDIR)
    }
    async fn delete_child(&self, _name: &str) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
    }
    fn poll(&self, req: &PollEvent, waker: Waker) -> PollEvent {
        let mut buffer = self.buffer.lock();
        let mut ret = PollEvent::empty();
        if self.is_read_end() {
            debug!("[PipeFile] poll read end, req: {:?}", req);
            if !buffer.has_writend() {
                debug!("[PipeFile] read end has no write end");
                ret |= PollEvent::POLLHUP;
            }
            if req.contains(PollEvent::POLLIN) && buffer.read_available() > 0 {
                debug!("[PipeFile] read end has data");
                ret |= PollEvent::POLLIN;
            } else {
                debug!("[PipeFile] read end has no data");
                buffer.add_read_event(waker);
            }
        } else {
            debug!("[PipeFile] poll write end, req: {:?}", req);
            if !buffer.has_readend() {
                debug!("[PipeFile] write end has no read end");
                ret |= PollEvent::POLLERR;
            }
            if req.contains(PollEvent::POLLOUT) && buffer.write_available() > 0 {
                debug!("[PipeFile] write end has space");
                ret |= PollEvent::POLLOUT;
            } else {
                debug!("[PipeFile] write end has no space");
                buffer.add_write_event(waker);
            }
        }
        ret
    }
}

impl Drop for PipeFile {
    fn drop(&mut self) {
        let mut buffer = self.buffer.lock();
        if self.is_read_end() {
            let name = self.meta.dentry().name();
            println!("[PipeFile] {} dropped!", name);
            root_dentry().remove_child(&name);
            buffer.read_end = None;
            for waker in buffer.write_wakers.drain(..) {
                waker.wake();
            }
        } else {
            let name = self.meta.dentry().name();
            println!("[PipeFile] {} dropped!", name);
            root_dentry().remove_child(&name);
            buffer.write_end = None;
            for waker in buffer.read_wakers.drain(..) {
                waker.wake();
            }
        }
    }
}

/// Pipe read future, only continue if it can be read
struct PipeReadFuture {
    pipe_buffer: Arc<SpinLock<PipeBuffer>>,
}

impl PipeReadFuture {
    fn new(pipe_buffer: Arc<SpinLock<PipeBuffer>>) -> Self {
        Self { pipe_buffer }
    }
}

impl Future for PipeReadFuture {
    // just for error handling
    type Output = SysResult<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut buffer = self.pipe_buffer.lock();
        if buffer.has_writend() {
            // only continue if it can be read
            if buffer.read_available() > 0 {
                Poll::Ready(Ok(()))
            } else {
                // ? will add multiple wakers?
                buffer.add_read_event(cx.waker().clone());
                Poll::Pending
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

/// Pipe write future, only continue if it can be written
struct PipeWriteFuture {
    pipe_buffer: Arc<SpinLock<PipeBuffer>>,
}

impl PipeWriteFuture {
    fn new(pipe_buffer: Arc<SpinLock<PipeBuffer>>) -> Self {
        Self { pipe_buffer }
    }
}

impl Future for PipeWriteFuture {
    // just for error handling
    type Output = SysResult<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut buffer = self.pipe_buffer.lock();
        if buffer.has_readend() {
            trace!("[PipeWriteFile] has read end");
            // only continue if it can be written
            if buffer.write_available() > 0 {
                trace!("[PipeWriteFile] write available");
                Poll::Ready(Ok(()))
            } else {
                trace!("[PipeWriteFile] write pending, save waker");
                // ? will add multiple wakers?
                buffer.add_write_event(cx.waker().clone());
                Poll::Pending
            }
        } else {
            Poll::Ready(Err(Errno::EPIPE))
        }
    }
}
