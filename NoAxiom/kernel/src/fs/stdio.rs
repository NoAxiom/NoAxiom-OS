use alloc::{boxed::Box, sync::Arc, vec::Vec};

use async_trait::async_trait;
use ksync::mutex::SpinLock;

use super::vfs::basic::file::{File, FileMeta};
use crate::{
    cpu::{current_cpu, get_hartid},
    include::result::Errno,
    syscall::SyscallResult,
};

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
    fn is_stdio(&self) -> bool {
        true
    }
    fn meta(&self) -> &FileMeta {
        unreachable!()
    }
    async fn base_read(&self, _offset: usize, buf: &mut [u8]) -> SyscallResult {
        // mention that getchar is busy loop
        let mut c = platform::getchar() as i8;
        loop {
            if c != -1 {
                break;
            }
            c = platform::getchar() as i8;
        }
        buf[0] = c as u8;
        Ok(1 as isize)
    }
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        Err(Errno::ENOSYS)
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
    async fn delete_child(&self, _name: &str) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
}

#[async_trait]
impl File for Stdout {
    fn is_stdio(&self) -> bool {
        true
    }
    fn meta(&self) -> &FileMeta {
        unreachable!()
    }
    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        Err(Errno::ENOSYS)
    }
    async fn base_write(&self, _offset: usize, buf: &[u8]) -> SyscallResult {
        // print!("{}", core::str::from_utf8(buf).unwrap());
        let mut stdout_buf = self.buf.lock();
        for c in buf.iter() {
            stdout_buf.push(*c);
            if *c == '\n' as u8 {
                print!(
                    "[PRINT, HART{}, TID{}] {}",
                    get_hartid(),
                    current_cpu()
                        .task
                        .as_ref()
                        .map_or_else(|| 0, |task| task.tid()),
                    core::str::from_utf8(stdout_buf.as_slice()).unwrap(),
                );
                stdout_buf.clear();
            }
        }
        Ok(buf.len() as isize)
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
    async fn delete_child(&self, _name: &str) -> Result<(), Errno> {
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
