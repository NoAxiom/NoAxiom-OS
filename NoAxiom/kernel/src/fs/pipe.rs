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

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::Ordering,
    task::{Context, Poll, Waker},
};

use async_trait::async_trait;
use ksync::mutex::SpinLock;
use memory::frame::{frame_alloc, FrameTracker};

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
    cpu::current_task,
    fs::vfs::basic::inode::Inode,
    include::{
        fs::{FileFlags, InodeMode, Stat},
        io::PollEvent,
        result::Errno,
    },
    signal::{
        sig_info::{SigCode, SigInfo},
        signal::Signal,
    },
    syscall::{SysResult, SyscallResult},
    utils::global_alloc,
};

#[derive(PartialEq)]
enum PipeBufferStatus {
    Empty,
    Full,
    Normal,
}

/// Ring buffer that max size is PIPE_BUF_SIZE
struct PipeBuffer {
    data: FrameTracker,
    head: usize,
    tail: usize,
    status: PipeBufferStatus,
    read_wakers: Vec<(usize, Waker)>,
    write_wakers: Vec<(usize, Waker)>,
    read_end: bool,
    write_end: bool,
}

impl PipeBuffer {
    fn new() -> Self {
        Self {
            data: frame_alloc().unwrap(),
            head: 0,
            tail: 0,
            status: PipeBufferStatus::Empty,
            read_wakers: Vec::new(),
            write_wakers: Vec::new(),
            read_end: false,
            write_end: false,
        }
    }
    fn add_read_event(&mut self, read_len: usize, waker: Waker) {
        self.read_wakers.push((read_len, waker));
    }
    fn add_write_event(&mut self, write_len: usize, waker: Waker) {
        self.write_wakers.push((write_len, waker));
    }
    fn read_available(&self) -> bool {
        match self.status {
            PipeBufferStatus::Empty => false,
            PipeBufferStatus::Full => true,
            PipeBufferStatus::Normal => true,
        }
    }
    fn write_available(&self) -> bool {
        match self.status {
            PipeBufferStatus::Empty => true,
            PipeBufferStatus::Full => false,
            PipeBufferStatus::Normal => true,
        }
    }
    fn read_available_len(&self) -> usize {
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
    fn write_available_len(&self) -> usize {
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
        let mut read_available = self.read_available_len();
        if read_available == 0 {
            return;
        }
        while let Some((len, waker)) = self.read_wakers.pop() {
            if read_available >= len {
                read_available -= len;
                waker.wake();
            } else {
                waker.wake();
                break;
            }
        }
    }
    fn notify_write_waker(&mut self) {
        let mut write_available = self.write_available_len();
        if write_available == 0 {
            return;
        }
        while let Some((len, waker)) = self.write_wakers.pop() {
            if write_available >= len {
                write_available -= len;
                waker.wake();
            } else if write_available > 0 {
                waker.wake();
                break;
            }
        }
    }
    /// Read `len` bytes as much as possible from the buffer, make sure buffer's
    /// size >= len, return the number of bytes read
    fn read(&mut self, buf: &mut [u8]) -> usize {
        trace!("[PipeBuffer] read buf");
        let len = buf.len();
        let data = self.data.ppn().get_bytes_array();
        let res = match self.status {
            PipeBufferStatus::Empty => 0,
            _ => {
                if self.head < self.tail {
                    let res = core::cmp::min(len, self.tail - self.head);
                    buf[..res].copy_from_slice(&data[self.head..self.head + res]);
                    res
                } else {
                    // maybe full
                    let res = core::cmp::min(len, PIPE_BUF_SIZE - self.head + self.tail);
                    if res <= PIPE_BUF_SIZE - self.head {
                        buf[..res].copy_from_slice(&data[self.head..self.head + res]);
                    } else {
                        buf[..PIPE_BUF_SIZE - self.head].copy_from_slice(&data[self.head..]);
                        buf[PIPE_BUF_SIZE - self.head..res]
                            .copy_from_slice(&data[..self.head + res - PIPE_BUF_SIZE]);
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
        let data = self.data.ppn().get_bytes_array();
        let res = match self.status {
            PipeBufferStatus::Full => 0,
            _ => {
                if self.head <= self.tail {
                    // maybe empty
                    trace!("[PipeBuffer] write maybe empty");
                    let res = core::cmp::min(len, self.head + PIPE_BUF_SIZE - self.tail);
                    if res <= PIPE_BUF_SIZE - self.tail {
                        data[self.tail..self.tail + res].copy_from_slice(&buf[..res]);
                    } else {
                        data[self.tail..].copy_from_slice(&buf[..PIPE_BUF_SIZE - self.tail]);
                        data[..self.tail + res - PIPE_BUF_SIZE]
                            .copy_from_slice(&buf[PIPE_BUF_SIZE - self.tail..res]);
                    }
                    res
                } else {
                    debug!("[PipeBuffer] write normal");
                    let res = core::cmp::min(len, self.head - self.tail);
                    data[self.tail..self.tail + res].copy_from_slice(&buf[..res]);
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

impl Drop for PipeBuffer {
    fn drop(&mut self) {
        debug!(
            "[PipeBuffer] dropped!! has_readend: {}, has_writend: {}",
            self.read_end, self.write_end
        );
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

    fn open(self: Arc<Self>, _file_flags: &FileFlags) -> SysResult<Arc<dyn File>> {
        unreachable!("pipe dentry should not open");
    }

    async fn create(self: Arc<Self>, _name: &str, _mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        unreachable!("pipe dentry should not create child");
    }

    async fn symlink(self: Arc<Self>, _name: &str, _tar_name: &str) -> SysResult<()> {
        unreachable!("pipe dentry shouldn't create symlink");
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
            st_mode: self.meta.inode_mode.load(Ordering::SeqCst),
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
    fn new_read_end(
        buffer: Arc<SpinLock<PipeBuffer>>,
        name: &str,
        file_flags: &FileFlags,
    ) -> Arc<Self> {
        let name = format!("{}-read", name);
        let dentry = PipeDentry::new(&name);
        let inode = Arc::new(PipeInode::new());
        dentry.set_inode(inode.clone());
        let meta = FileMeta::new(dentry, inode, &(*file_flags | FileFlags::O_RDONLY));
        let res = Arc::new(Self { buffer, meta });
        res
    }
    fn new_write_end(
        buffer: Arc<SpinLock<PipeBuffer>>,
        name: &str,
        file_flags: &FileFlags,
    ) -> Arc<Self> {
        let name = format!("{}-write", name);
        let dentry = PipeDentry::new(&name);
        let inode = Arc::new(PipeInode::new());
        dentry.set_inode(inode.clone());
        let meta = FileMeta::new(dentry, inode, &(*file_flags | FileFlags::O_WRONLY));
        let res = Arc::new(Self { buffer, meta });
        res
    }
    fn is_read_end(&self) -> bool {
        self.meta.readable()
    }
    fn is_write_end(&self) -> bool {
        self.meta.writable()
    }
    /// Create a new pipe, return (read end, write end)
    pub fn new_pipe(file_flags: &FileFlags) -> (Arc<Self>, Arc<Self>) {
        let buffer = Arc::new(SpinLock::new(PipeBuffer::new()));
        let name = format!("pipe-{}", global_alloc());
        let read_end = Self::new_read_end(buffer.clone(), &name, file_flags);
        let write_end = Self::new_write_end(buffer.clone(), &name, file_flags);
        buffer.lock().read_end = true;
        buffer.lock().write_end = true;
        (read_end, write_end)
    }
}

#[async_trait]
impl File for PipeFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    fn is_interruptable(&self) -> bool {
        true
    }
    async fn base_read(&self, _offset: usize, buf: &mut [u8]) -> SyscallResult {
        assert!(self.is_read_end());
        debug!("[pipe] {} read, {}", self.meta.dentry().name(), buf.len());
        PipeReadFuture::new(buf.len(), self.buffer.clone()).await?;
        let mut buffer = self.buffer.lock();
        let ret = buffer.read(buf);
        if ret != 0 {
            buffer.notify_write_waker();
        }
        Ok(ret as isize)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, buf: &[u8]) -> SyscallResult {
        assert!(self.is_write_end());
        debug!("[pipe] {} write, {}", self.meta.dentry().name(), buf.len());
        PipeWriteFuture::new(buf.len(), self.buffer.clone()).await?;
        let mut buffer = self.buffer.lock();
        let ret = buffer.write(buf);
        if ret != 0 {
            buffer.notify_read_waker();
        }
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
            if !buffer.write_end {
                debug!("[PipeFile] read end has no write end");
                ret |= PollEvent::POLLHUP;
            }
            if req.contains(PollEvent::POLLIN) && buffer.read_available() {
                debug!("[PipeFile] read end has data");
                ret |= PollEvent::POLLIN;
            } else {
                debug!("[PipeFile] read end has no data");
                buffer.add_read_event(0, waker);
            }
        } else {
            debug!("[PipeFile] poll write end, req: {:?}", req);
            if !buffer.read_end {
                debug!("[PipeFile] write end has no read end");
                ret |= PollEvent::POLLERR;
            }
            if req.contains(PollEvent::POLLOUT) && buffer.write_available() {
                debug!("[PipeFile] write end has space");
                ret |= PollEvent::POLLOUT;
            } else {
                debug!("[PipeFile] write end has no space");
                buffer.add_write_event(0, waker);
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
            warn!("[PipeFile] {} dropped!", name);
            root_dentry().remove_child(&name);
            buffer.read_end = false;
            for (_, waker) in buffer.write_wakers.drain(..) {
                waker.wake();
            }
        } else {
            let name = self.meta.dentry().name();
            warn!("[PipeFile] {} dropped!", name);
            root_dentry().remove_child(&name);
            buffer.write_end = false;
            for (_, waker) in buffer.read_wakers.drain(..) {
                waker.wake();
            }
        }
    }
}

/// Pipe read future, only continue if it can be read
struct PipeReadFuture {
    read_len: usize,
    pipe_buffer: Arc<SpinLock<PipeBuffer>>,
}

impl PipeReadFuture {
    fn new(read_len: usize, pipe_buffer: Arc<SpinLock<PipeBuffer>>) -> Self {
        Self {
            read_len,
            pipe_buffer,
        }
    }
}

impl Future for PipeReadFuture {
    // just for error handling
    type Output = SysResult<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut buffer = self.pipe_buffer.lock();
        if buffer.write_end {
            // only continue if it can be read
            if buffer.read_available() {
                Poll::Ready(Ok(()))
            } else {
                buffer.add_read_event(self.read_len, cx.waker().clone());
                Poll::Pending
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

/// Pipe write future, only continue if it can be written
struct PipeWriteFuture {
    write_len: usize,
    pipe_buffer: Arc<SpinLock<PipeBuffer>>,
}

impl PipeWriteFuture {
    fn new(write_len: usize, pipe_buffer: Arc<SpinLock<PipeBuffer>>) -> Self {
        Self {
            write_len,
            pipe_buffer,
        }
    }
}

impl Future for PipeWriteFuture {
    // just for error handling
    type Output = SysResult<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut buffer = self.pipe_buffer.lock();
        if buffer.read_end {
            trace!("[PipeWriteFile] has read end");
            // only continue if it can be written
            if buffer.write_available() {
                trace!("[PipeWriteFile] write available");
                Poll::Ready(Ok(()))
            } else {
                trace!("[PipeWriteFile] write pending, save waker");
                buffer.add_write_event(self.write_len, cx.waker().clone());
                Poll::Pending
            }
        } else {
            // No read end - send SIGPIPE to current process
            warn!("[PipeWriteFile] no read end, sending SIGPIPE");
            if let Some(task) = current_task() {
                let siginfo = SigInfo::new_simple(Signal::SIGPIPE, SigCode::Kernel);
                task.recv_siginfo(siginfo, false);
            }
            Poll::Ready(Err(Errno::EPIPE))
        }
    }
}
