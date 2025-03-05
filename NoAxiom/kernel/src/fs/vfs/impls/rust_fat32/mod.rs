use disk_cursor::DiskCursor;
use fatfs::{DefaultTimeProvider, Dir, Error, File, FileSystem, LossyOemCpConverter};

use crate::{include::result::Errno, syscall::SyscallResult};

pub mod dentry;
mod disk_cursor;
pub mod file;
pub mod filesystem;
pub mod inode;
pub mod superblock;

/// the implementation of FAT32 file system
type IFatFileDir = Dir<DiskCursor, DefaultTimeProvider, LossyOemCpConverter>;
type IFatFileFile = File<DiskCursor, DefaultTimeProvider, LossyOemCpConverter>;
type IFatFs = FileSystem<DiskCursor, DefaultTimeProvider, LossyOemCpConverter>;

pub const fn fs_err(err: Error<()>) -> Errno {
    match err {
        Error::NotFound => Errno::ENOENT,
        _ => Errno::EIO,
    }
}
