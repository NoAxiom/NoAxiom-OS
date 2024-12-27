//! for os use: Inode
//! provide the interface for file system

use alloc::{boxed::Box, string::String, vec::Vec};
use core::{future::Future, pin::Pin};

use async_trait::async_trait;

use super::{path::Path, File};
use crate::fs::FS;

pub struct Inode<T> {
    pub readable: bool,
    pub writable: bool,
    identifier: T,
}

impl Inode<String> {
    pub fn from(identifier: Path) -> Self {
        Self {
            readable: true,
            writable: false,
            identifier: identifier.as_string(),
        }
    }
}

#[async_trait]
impl<T: Send + Sync> File for Inode<T>
where
    String: From<T>,
    T: Clone,
{
    async fn read_part<'a>(&'a self, offset: usize, len: usize, buf: &'a mut [u8]) {
        let gaurd = FS.lock();
        let fs = gaurd.as_ptr();
        let fs = unsafe { &*fs };
        fs.load_file_part(self.identifier.clone().into(), offset, len, buf)
            .await;
    }
    async fn write<'a>(&'a self, buf: &'a [u8]) {
        let gaurd = FS.lock();
        let fs = gaurd.as_ptr();
        let fs = unsafe { &*fs };
        todo!();
    }
    fn read<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ()>> + Send + 'a>> {
        Box::pin(async move {
            let gaurd = FS.lock();
            let fs = gaurd.as_ptr();
            let fs = unsafe { &*fs };
            Ok(fs.load_file(self.identifier.clone().into()).await)
        })
    }
}
