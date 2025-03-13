use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;

use super::{dentry::Ext4Dentry, inode::Ext4FileInode, superblock::Ext4SuperBlock};
use crate::{
    fs::vfs::{
        basic::{
            file::{File, FileMeta},
            inode::Inode,
        },
        impls::ext4::{fs_err, inode::Ext4DirInode},
    },
    include::{
        fs::{Ext4DirEntryType, InodeMode},
        result::Errno,
    },
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
            ino: inode.get_inode().lock().inode_num,
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
        let inode = &self.meta.inode;
        let super_block = self.meta.dentry().super_block();
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs();

        match inode.file_type() {
            InodeMode::FILE => {
                Ok(ext4.read_at(self.ino, offset, buf).await.map_err(fs_err)? as isize)
            }
            InodeMode::DIR => {
                return Err(Errno::EISDIR);
            }
            _ => unreachable!(),
        }
    }

    /// write all the buf content, extend the file if necessary
    async fn base_write(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        let inode = &self.meta.inode;
        let super_block = self.meta.dentry().super_block();
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs();

        match inode.file_type() {
            InodeMode::FILE => {
                Ok(ext4.write_at(self.ino, offset, buf).await.map_err(fs_err)? as isize)
            }
            InodeMode::DIR => {
                return Err(Errno::EISDIR);
            }
            _ => unreachable!(),
        }
    }
    async fn load_dir(&self) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
    }
    async fn delete_child(&self, _name: &str) -> Result<(), Errno> {
        Err(Errno::ENOSYS)
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

    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        Err(Errno::EISDIR)
    }

    async fn load_dir(&self) -> Result<(), Errno> {
        debug!("[AsyncSmpExt4]FIle: load_dir");
        let super_block = self.meta.dentry().super_block();
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs();

        let entries = ext4.dir_get_entries(self.ino).await;
        let self_path = self.meta().dentry().path().as_string();

        for entry in entries {
            let child_name = entry.get_name();
            if child_name == "." || child_name == ".." {
                debug!("load_dir: {:?}, passed", child_name);
                continue;
            }
            let child_path = if self_path != "/" {
                format!("{}/{}", self_path, entry.get_name())
            } else {
                format!("/{}", entry.get_name())
            };
            let child_inode: Arc<dyn Inode> =
                if entry.get_de_type() == Ext4DirEntryType::EXT4_DE_DIR.bits() {
                    debug!("load_dir: {:?}", child_name);
                    let inode_num = ext4.ext4_dir_open(&child_path).await.map_err(fs_err)?;
                    let inode = ext4.get_inode_ref(inode_num).await;
                    Arc::new(Ext4DirInode::new(super_block.clone(), inode))
                } else if entry.get_de_type() == Ext4DirEntryType::EXT4_DE_REG_FILE.bits() {
                    debug!("load_file: {:?}", child_name);
                    let inode_num = ext4
                        .ext4_file_open(&child_path, "r+")
                        .await
                        .map_err(fs_err)?;
                    let inode = ext4.get_inode_ref(inode_num).await;
                    Arc::new(Ext4FileInode::new(super_block.clone(), inode))
                } else {
                    unreachable!();
                };
            self.dentry().add_child(&child_name, child_inode);
        }
        Ok(())
    }

    async fn delete_child(&self, name: &str) -> Result<(), Errno> {
        let super_block = self.meta.dentry().super_block();
        let ext4 = super_block
            .downcast_ref::<Ext4SuperBlock>()
            .unwrap()
            .get_fs();
        let mut inode = ext4.get_inode_ref(self.ino).await;
        let self_path = self.meta().dentry().path().as_string();
        let child_path = if self_path != "/" {
            format!("{}/{}", self_path, name)
        } else {
            format!("/{}", name)
        };
        ext4.dir_remove_entry(&mut inode, &child_path)
            .await
            .map_err(fs_err)?;
        Ok(())
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
