use alloc::{boxed::Box, sync::Arc, vec::Vec};

use arch::{Arch, ArchSbi};
use async_trait::async_trait;
use ksync::mutex::SpinLock;

use super::vfs::basic::file::{File, FileMeta};
use crate::{include::result::Errno, syscall::SyscallResult};

pub struct Stdin;
pub struct Stdout {
    buf: Arc<SpinLock<Vec<u8>>>,
}

impl Stdout {
    pub fn new() -> Self {
        Self {
            buf: Arc::new(SpinLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl File for Stdin {
    fn meta(&self) -> &FileMeta {
        unreachable!()
    }
    async fn read_from<'a>(&'a self, _offset: usize, buf: &'a mut Vec<u8>) -> SyscallResult {
        // mention that getchar is busy loop
        let mut c = Arch::console_getchar() as i8;
        loop {
            if c != -1 {
                break;
            }
            c = Arch::console_getchar() as i8;
        }
        buf[0] = c as u8;
        Ok(1 as isize)
    }
    async fn write_at<'a>(&'a self, _offset: usize, _buf: &'a Vec<u8>) -> SyscallResult {
        Err(Errno::ENOSYS)
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
}

#[async_trait]
impl File for Stdout {
    fn meta(&self) -> &FileMeta {
        unreachable!()
    }
    async fn read_from<'a>(&'a self, _offset: usize, _buf: &'a mut Vec<u8>) -> SyscallResult {
        Err(Errno::ENOSYS)
    }
    async fn write_at<'a>(&'a self, _offset: usize, user_buf: &'a Vec<u8>) -> SyscallResult {
        // print!("{}", core::str::from_utf8(buf).unwrap());
        let mut stdout_buf = self.buf.lock();
        for c in user_buf.iter() {
            stdout_buf.push(*c);
            if *c == '\n' as u8 {
                print!("{}", core::str::from_utf8(stdout_buf.as_slice()).unwrap());
                stdout_buf.clear();
            }
        }
        Ok(user_buf.len() as isize)
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
}

impl Drop for Stdout {
    fn drop(&mut self) {
        let stdout_buf = self.buf.lock();
        if !stdout_buf.is_empty() {
            print!("{}", core::str::from_utf8(stdout_buf.as_slice()).unwrap());
        }
    }
}
