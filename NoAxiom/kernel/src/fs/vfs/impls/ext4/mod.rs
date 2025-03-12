use ext4_rs::{Errno as ext4Errno, Ext4Error};

use crate::include::result::Errno;

pub mod dentry;
mod disk_cursor;
pub mod file;
pub mod filesystem;
pub mod inode;
pub mod superblock;

type IExtFs = ext4_rs::Ext4;
type IExtInode = ext4_rs::ext4_defs::Ext4InodeRef;

pub const fn fs_err(err: Ext4Error) -> Errno {
    let err_value = err.error();
    match err_value {
        ext4Errno::ENOENT => Errno::ENOENT,
        ext4Errno::EIO => Errno::EIO,
        ext4Errno::ENOSPC => Errno::ENOSPC,
        ext4Errno::ENOTDIR => Errno::ENOTDIR,
        ext4Errno::EISDIR => Errno::EISDIR,
        ext4Errno::EINVAL => Errno::EINVAL,
        // todo: add more error mapping
        _ => Errno::EIO,
    }
}
