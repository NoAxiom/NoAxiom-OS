use alloc::{boxed::Box, sync::Arc};
use core::task::Waker;

use async_trait::async_trait;
use fatfs::{Read, Seek, SeekFrom::Start, Write};
use spin::Mutex;

use super::{
    dentry::Fat32Dentry,
    fs_err,
    inode::{Fat32DirInode, Fat32FileInode},
    IFatFileDir, IFatFileFile,
};
use crate::{
    fs::vfs::basic::{
        file::{File, FileMeta},
        inode::Inode,
    },
    include::{
        fs::{FileFlags, InodeMode},
        io::PollEvent,
        result::Errno,
    },
    syscall::SyscallResult,
};

pub struct Fat32File {
    meta: FileMeta,
    inner: Arc<Mutex<IFatFileFile>>,
}

impl Fat32File {
    pub fn new(
        dentry: Arc<Fat32Dentry>,
        inode: Arc<Fat32FileInode>,
        file_flags: &FileFlags,
    ) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone(), file_flags),
            inner: inode.get_file(),
        }
    }
}

#[async_trait]
impl File for Fat32File {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }

    // offset:
    //  - offset == cursor.offset: normal read
    //  - offset != cursor.offset: seek and read
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let inode = &self.meta.inode;
        match inode.file_type() {
            InodeMode::FILE => {
                let mut inner = self.inner.lock();
                let fat_offset = inner.offset() as usize;
                if offset != fat_offset {
                    inner.seek(Start(offset as u64)).map_err(fs_err)?;
                }
                inner.read_exact(buf).await.map_err(fs_err)?;

                let readsize = (inner.size().unwrap() as usize - fat_offset).min(buf.len());
                Ok(readsize as isize)
            }
            InodeMode::DIR => {
                return Err(Errno::EISDIR);
            }
            _ => unreachable!(),
        }
    }

    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }

    /// write all the buf content, extend the file if necessary
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        let inode = &self.meta.inode;
        let size = inode.size();
        match inode.file_type() {
            InodeMode::FILE => {
                let mut inner = self.inner.lock();

                if offset > size {
                    let empty = vec![0; offset - size];
                    inner.seek(Start(size as u64)).map_err(fs_err)?;
                    inner.write_all(&empty).await.map_err(fs_err)?;
                }

                let fat_offset = inner.offset() as usize;
                if offset != fat_offset {
                    inner.seek(Start(offset as u64)).map_err(fs_err)?;
                }
                inner.write_all(buf).await.map_err(fs_err)?;

                if offset + buf.len() > size {
                    inode.set_size(offset + buf.len());
                }

                Ok(buf.len() as isize)
            }
            InodeMode::DIR => {
                return Err(Errno::EISDIR);
            }
            _ => unreachable!(),
        }
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
    fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
        unreachable!("Fat32file::poll not supported now");
    }
}

pub struct Fat32Dir {
    meta: FileMeta,
    inner: Arc<Mutex<IFatFileDir>>,
}

impl Fat32Dir {
    pub fn new(
        dentry: Arc<Fat32Dentry>,
        inode: Arc<Fat32DirInode>,
        file_flags: &FileFlags,
    ) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone(), file_flags),
            inner: inode.get_dir(),
        }
    }
}

#[async_trait]
impl File for Fat32Dir {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }

    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        Err(Errno::EISDIR)
    }

    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }

    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        Err(Errno::EISDIR)
    }

    async fn load_dir(&self) -> Result<(), Errno> {
        debug!("[AsyncSmpFat32]FIle: load_dir");
        let super_block = self.meta().dentry().super_block();
        let mut iter = self.inner.lock().iter();
        while let Some(entry) = iter.next() {
            let Ok(entry) = entry else {
                return Err(Errno::EIO);
            };
            let child_inode: Arc<dyn Inode> = if entry.is_dir() {
                debug!("load_dir: {:?}", entry.file_name());
                Arc::new(Fat32DirInode::new(super_block.clone(), entry.to_dir()))
            } else if entry.is_file() {
                debug!("load_file: {:?}", entry.file_name());
                Arc::new(Fat32FileInode::new(super_block.clone(), entry.to_file()))
            } else {
                unreachable!();
            };
            self.dentry()
                .add_child_with_inode(&entry.file_name(), child_inode);
        }
        Ok(())
    }

    async fn delete_child(&self, name: &str) -> Result<(), Errno> {
        let inner = self.inner.lock();
        inner.remove(name).await.map_err(fs_err)?;
        Ok(())
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
    }
    fn poll(&self, _req: &PollEvent, _waker: Waker) -> PollEvent {
        unreachable!("Fat32dir::poll not supported now");
    }
}
