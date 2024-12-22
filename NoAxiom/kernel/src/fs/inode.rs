//! for os use: Inode
//! provide the interface for file system

use alloc::{boxed::Box, string::String};

use super::{File, FileReturn};
use crate::{config::errno::Errno, fs::FS};

pub struct Inode<T> {
    pub readable: bool,
    pub writable: bool,
    identifier: T,
}

impl Inode<String> {
    pub fn from(identifier: String) -> Self {
        Self {
            readable: true,
            writable: false,
            identifier,
        }
    }
}

impl<T: Send + Sync> File for Inode<T>
where
    String: From<T>,
    T: Clone,
{
    fn read<'a>(&self) -> FileReturn {
        Box::pin(async move {
            let gaurd = FS.lock();
            let fs = gaurd.as_ptr();
            let fs = unsafe { &*fs };
            Ok(fs.load_file(self.identifier.clone().into()).await)
        })
    }

    fn write<'a>(&'a self, buf: &'a [u8]) -> FileReturn {
        Box::pin(async move {
            panic!("write not implemented");
        })
    }

    fn flush(&self) -> Result<(), ()> {
        Err(())
    }

    fn close(&self) -> Result<(), ()> {
        Err(())
    }
}
