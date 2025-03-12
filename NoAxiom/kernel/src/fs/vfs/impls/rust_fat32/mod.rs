use fatfs::{DefaultTimeProvider, Dir, Error, File, FileSystem, LossyOemCpConverter};

use super::disk_cursor::DiskCursor;
use crate::include::result::Errno;

pub mod dentry;
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
