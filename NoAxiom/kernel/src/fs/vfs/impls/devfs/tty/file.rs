use alloc::{boxed::Box, vec::Vec};
use core::task::Waker;

use async_trait::async_trait;
use include::errno::Errno;
use ksync::mutex::SpinLock;

use crate::{
    driver::base::getchar,
    fs::vfs::basic::file::{File, FileMeta},
    include::{
        fs::{
            Termios,
            TtyIoctlCmd::{self, *},
            WinSize,
        },
        io::PollEvent,
    },
    sched::utils::yield_now,
    syscall::{SysResult, SyscallResult},
};

type Pid = u32;

struct TtyInner {
    fg_pgid: Pid,
    win_size: WinSize,
    termios: Termios,
}

pub struct TtyFile {
    meta: FileMeta,
    inner: SpinLock<TtyInner>,
    buf: SpinLock<Vec<u8>>,
}

impl TtyFile {
    pub fn new(meta: FileMeta) -> Self {
        Self {
            meta,
            inner: SpinLock::new(TtyInner {
                fg_pgid: 1 as u32,
                win_size: WinSize::new(),
                termios: Termios::new(),
            }),
            buf: SpinLock::new(Vec::new()),
        }
    }
}

#[async_trait]
impl File for TtyFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!("readlink from tty");
    }
    async fn base_read(&self, _offset: usize, buf: &mut [u8]) -> SyscallResult {
        let mut c = getchar() as i8;
        loop {
            if c != -1 {
                break;
            } else {
                yield_now().await;
                c = getchar() as i8;
            }
        }
        buf[0] = c as u8;
        Ok(1 as isize)
    }
    async fn base_write(&self, _offset: usize, buf: &[u8]) -> SyscallResult {
        #[cfg(feature = "log_print")]
        {
            let mut stdout_buf = self.buf.lock();
            for c in buf.iter() {
                stdout_buf.push(*c);
                if *c == '\n' as u8 {
                    // only for debug
                    print!(
                        "[PRINT, HART{}, TID{} at {}ms] ",
                        crate::cpu::get_hartid(),
                        crate::cpu::current_cpu()
                            .task
                            .as_ref()
                            .map_or_else(|| 0, |task| task.tid()),
                        crate::time::gettime::get_time_ms(),
                    );
                    for it in stdout_buf.iter() {
                        print!("{}", *it as char);
                    }
                    stdout_buf.clear();
                }
            }
        }
        #[cfg(not(feature = "log_print"))]
        {
            print!("{}", core::str::from_utf8(buf).unwrap());
        }
        Ok(buf.len() as isize)
    }

    async fn load_dir(&self) -> SysResult<()> {
        Err(Errno::ENOTDIR)
    }
    async fn delete_child(&self, _name: &str) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }
    fn ioctl(&self, cmd: usize, arg: usize) -> SyscallResult {
        let Some(cmd) = TtyIoctlCmd::from_repr(cmd) else {
            log::error!("[TtyFile::ioctl] cmd {cmd} not included");
            unimplemented!()
        };
        log::info!("[TtyFile::ioctl] cmd {:?}, value {:#x}", cmd, arg);
        match cmd {
            TCGETS | TCGETA => {
                unsafe {
                    *(arg as *mut Termios) = self.inner.lock().termios;
                }
                Ok(0)
            }
            TCSETS | TCSETSW | TCSETSF => {
                unsafe {
                    self.inner.lock().termios = *(arg as *const Termios);
                    log::info!("termios {:#x?}", self.inner.lock().termios);
                }
                Ok(0)
            }
            TIOCGPGRP => {
                let fg_pgid = self.inner.lock().fg_pgid;
                log::info!("[TtyFile::ioctl] get fg pgid {fg_pgid}");
                unsafe {
                    *(arg as *mut Pid) = fg_pgid;
                }
                Ok(0)
            }
            TIOCSPGRP => {
                unsafe {
                    self.inner.lock().fg_pgid = *(arg as *const Pid);
                }
                let fg_pgid = self.inner.lock().fg_pgid;
                log::info!("[TtyFile::ioctl] set fg pgid {fg_pgid}");
                Ok(0)
            }
            TIOCGWINSZ => {
                let win_size = self.inner.lock().win_size;
                log::info!("[TtyFile::ioctl] get window size {win_size:?}",);
                unsafe {
                    *(arg as *mut WinSize) = win_size;
                }
                Ok(0)
            }
            TIOCSWINSZ => {
                unsafe {
                    self.inner.lock().win_size = *(arg as *const WinSize);
                }
                Ok(0)
            }
            TCSBRK => Ok(0),
            _ => todo!(),
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

impl Drop for TtyFile {
    fn drop(&mut self) {
        let stdout_buf = self.buf.lock();
        if !stdout_buf.is_empty() {
            print!("{}", core::str::from_utf8(stdout_buf.as_slice()).unwrap());
        }
    }
}
