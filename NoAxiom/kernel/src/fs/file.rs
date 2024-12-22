use alloc::{boxed::Box, vec::Vec};
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

pub type FileReturn<'a> = Pin<Box<dyn Future<Output = Result<Vec<u8>, Errno>> + Send + 'a>>;

pub trait File: Send + Sync {
    fn read<'a>(&'a self) -> FileReturn;
    fn write<'a>(&'a self, buf: &'a [u8]) -> FileReturn;
    fn flush(&self) -> Result<(), ()>;
    fn close(&self) -> Result<(), ()>;
}
