use super::vfs::basic::file::{File, FileMeta};
use crate::{include::result::Errno, syscall::SyscallResult};

pub struct Stdin;
pub struct Stdout;

use alloc::{boxed::Box, vec::Vec};

use async_trait::async_trait;
use sbi_rt::legacy::console_getchar;

#[async_trait]
impl File for Stdin {
    fn meta(&self) -> &FileMeta {
        unreachable!()
    }
    async fn read_from<'a>(&'a self, _offset: usize, buf: &'a mut Vec<u8>) -> SyscallResult {
        // mention that getchar is busy loop
        let mut c = console_getchar() as i8;
        loop {
            if c != -1 {
                break;
            }
            c = console_getchar() as i8;
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
    async fn write_at<'a>(&'a self, _offset: usize, buf: &'a Vec<u8>) -> SyscallResult {
        print!("{}", core::str::from_utf8(buf).unwrap());
        Ok(buf.len() as isize)
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
}
