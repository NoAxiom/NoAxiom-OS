use alloc::{boxed::Box, sync::Arc};
use core::task::Waker;

use arch::{Arch, ArchInt};
use async_trait::async_trait;
use ext4_rs::InodeFileType;
use ksync::mutex::check_no_lock;

use super::{dentry::Ext4Dentry, inode::Ext4FileInode, superblock::Ext4SuperBlock};
use crate::{
    fs::vfs::{
        basic::{
            file::{File, FileMeta},
            inode::Inode,
        },
        impls::ext4::{fs_err, inode::Ext4DirInode},
    },
    include::{fs::InodeMode, io::PollEvent, result::Errno},
    sched::utils::block_on,
    syscall::SyscallResult,
};

pub struct Ext4File {
    meta: FileMeta,
    /// EXT4_RS doesn't support File/Dir struct, so we use ino to represent the
    /// the file struct in ext4, multi threads read/write the same file should
    /// ensure the atomicity, which provided by the fs lock
    ino: u32,
}

impl Ext4File {
    pub fn new(dentry: Arc<Ext4Dentry>, inode: Arc<Ext4FileInode>) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone()),
            ino: block_on(inode.get_inode().lock()).inode_num,
        }
    }
}

#[async_trait]
impl File for Ext4File {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }

    // offset:
    //  - offset == cursor.offset: normal read
    //  - offset != cursor.offset: seek and read
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        assert!(check_no_lock());
        assert!(Arch::is_interrupt_enabled());
        if offset > self.meta.inode.size() {
            return Ok(0);
        }
        let inode = &self.meta.inode;
        let super_block = self.meta.dentry().super_block();
        trace!("[ext4file] read try to get lock");
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs()
            .await;
        trace!("[ext4file] read get lock succeed");

        match inode.file_type() {
            InodeMode::FILE => {
                assert!(check_no_lock());
                assert!(Arch::is_interrupt_enabled());
                let x = ext4.read_at(self.ino, offset, buf).await.map_err(fs_err)? as isize;
                Ok(x)
            }
            InodeMode::DIR => {
                return Err(Errno::EISDIR);
            }
            _ => unreachable!(),
        }
    }

    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        todo!()
    }

    /// write all the buf content, extend the file if necessary
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        assert!(check_no_lock());
        assert!(Arch::is_interrupt_enabled());
        let inode = &self.meta.inode;
        let super_block = self.meta.dentry().super_block();
        trace!("[ext4file] write try to get lock");
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs()
            .await;
        trace!("[ext4file] write get lock succeed");
        let size = inode.size();
        if offset + buf.len() > size {
            inode.set_size(offset + buf.len());
        }
        match inode.file_type() {
            InodeMode::FILE => {
                assert!(check_no_lock());
                assert!(Arch::is_interrupt_enabled());
                Ok(ext4.write_at(self.ino, offset, buf).await.map_err(fs_err)? as isize)
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

pub struct Ext4Dir {
    meta: FileMeta,
    /// EXT4_RS doesn't support File/Dir struct, so we use ino to represent the
    /// the dir struct in ext4, multi threads read/write the same file should
    /// ensure the atomicity, which provided by the fs lock
    ino: u32,
}

impl Ext4Dir {
    pub fn new(dentry: Arc<Ext4Dentry>, inode: Arc<Ext4DirInode>) -> Self {
        Self {
            meta: FileMeta::new(dentry.clone(), inode.clone()),
            ino: inode.get_inode().lock().inode_num,
        }
    }
}

#[async_trait]
impl File for Ext4Dir {
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
        error!("write to a dir");
        Err(Errno::EISDIR)
    }

    async fn load_dir(&self) -> Result<(), Errno> {
        static mut FIRST: bool = true;
        debug!("[AsyncSmpExt4]Dir {}: load_dir", self.meta.dentry().name());
        let super_block = self.meta.dentry().super_block();
        trace!("[ext4dir] load try to get lock");
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs()
            .await;
        trace!("[ext4dir] load get lock succeed");
        assert!(check_no_lock());
        assert!(Arch::is_interrupt_enabled());

        let entries = ext4.dir_get_entries(self.ino).await;
        for entry in entries {
            let entry_inode = ext4.get_inode_ref(entry.inode).await;
            let file_type = entry_inode.inode.file_type();
            let child_name = entry.get_name();
            if child_name == "." || child_name == ".." {
                if unsafe { FIRST } {
                    debug!("load {:?}: {} pass", file_type, child_name);
                }
                continue;
            }
            if unsafe { FIRST } {
                debug!("load {:?}: {}", file_type, child_name);
            }
            let inode: Arc<dyn Inode> = if file_type.contains(InodeFileType::S_IFREG) {
                Arc::new(Ext4FileInode::new(super_block.clone(), entry_inode))
            } else if file_type == InodeFileType::S_IFDIR {
                Arc::new(Ext4DirInode::new(super_block.clone(), entry_inode))
            } else {
                unreachable!(
                    "load_dir: unsupportable file {}: type {:?}",
                    child_name, file_type
                );
            };
            self.dentry().add_child(&child_name, inode);
        }
        unsafe {
            if FIRST {
                FIRST = false;
            }
        }
        Ok(())
    }

    async fn delete_child(&self, name: &str) -> Result<(), Errno> {
        let super_block = self.meta.dentry().super_block();
        debug!("[ext4dir] delete_child try to get lock");
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs()
            .await;
        debug!("[ext4dir] delete_child  get lock succeed");
        assert!(check_no_lock());
        assert!(Arch::is_interrupt_enabled());
        let mut inode = ext4.get_inode_ref(self.ino).await;
        ext4.dir_remove_entry(&mut inode, &name)
            .await
            .map_err(fs_err)?;
        Ok(())
    }

    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        Err(Errno::ENOTTY)
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

/*

/// Parse the file access flags (such as "r", "w", "a", etc.) and convert them to system constants.
fn ext4_parse_flags(&self, flags: &str) -> Result<i32> {
    match flags {
        "r" | "rb" => Ok(O_RDONLY),
        "w" | "wb" => Ok(O_WRONLY | O_CREAT | O_TRUNC),
        "a" | "ab" => Ok(O_WRONLY | O_CREAT | O_APPEND),
        "r+" | "rb+" | "r+b" => Ok(O_RDWR),
        "w+" | "wb+" | "w+b" => Ok(O_RDWR | O_CREAT | O_TRUNC),
        "a+" | "ab+" | "a+b" => Ok(O_RDWR | O_CREAT | O_APPEND),
        _ => Err(Ext4Error::new(Errno::EINVAL)),
    }
}

 */
