use alloc::boxed::Box;
use core::{future::Future, pin::Pin};

use crate::config::errno::Errno;

/// file's data
pub struct FileData<T> {
    pub inner: T,
}

impl<T> FileData<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

pub type FileReturn<'a> = Pin<Box<dyn Future<Output = Result<isize, Errno>> + Send + 'a>>;

pub trait File: Send + Sync {
    /// read from [`addr`, `addr` + `len`), write to `buf`
    fn read<'a>(&'a self, addr: usize, len: usize, buf: &'a mut [u8]) -> FileReturn;
    /// todo: write to file
    fn write<'a>(&'a self, addr: usize, buf: &'a [u8]) -> FileReturn;
    fn flush(&self) -> Result<(), ()>;
    fn close(&self) -> Result<(), ()>;
}
