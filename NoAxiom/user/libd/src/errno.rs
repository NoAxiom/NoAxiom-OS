//! Linux error number: https://man7.org/linux/man-pages/man3/errno.3.html

#![allow(unused)]

use thiserror::Error;

pub type Result = core::result::Result<isize, Errno>;

#[derive(Debug, Error)]
pub enum Errno {
    /// Error cannot be clairified due to current implementation
    ///
    /// # Note
    ///
    /// Should only used for debugging.
    #[error("Error cannot be clairified due to current implementation")]
    DISCARD = -1,

    /// Operation not permitted
    #[error("[EPERM] Operation not permitted")]
    EPERM = 1,

    /// No such file or directory
    #[error("[ENOENT] No such file or directory")]
    ENOENT = 2,

    /// No such process
    #[error("[ESRCH] No such process")]
    ESRCH = 3,

    /// Interrupted system call
    #[error("[EINTR] Interrupted system call")]
    EINTR = 4,

    /// I/O error
    #[error("[EIO] I/O error")]
    EIO = 5,

    /// No such device or address
    #[error("[ENXIO] No such device or address")]
    ENXIO = 6,

    /// Argument list too long
    #[error("[E2BIG] Argument list too long")]
    E2BIG = 7,

    /// Exec format error
    #[error("[ENOEXEC] Exec format error")]
    ENOEXEC = 8,

    /// Bad file number
    #[error("[EBADF] Bad file number")]
    EBADF = 9,

    /// No child processes
    #[error("[ECHILD] No child processes")]
    ECHILD = 10,

    /// Try again
    #[error("[EAGAIN] Try again")]
    EAGAIN = 11,

    /// Out of memory
    #[error("[ENOMEM] Out of memory")]
    ENOMEM = 12,

    /// Permission denied
    #[error("[EACCES] Permission denied2")]
    EACCES = 13,

    /// Bad address
    #[error("[EFAULT] Bad address")]
    EFAULT = 14,

    /// Block device required
    #[error("[ENOTBLK] Block device required")]
    ENOTBLK = 15,

    /// Device or resource busy
    #[error("[EBUSY] Device or resource busy")]
    EBUSY = 16,

    /// File exists
    #[error("[EEXIST] File exists")]
    EEXIST = 17,

    /// Cross-device link
    #[error("[EXDEV] Cross-device link")]
    EXDEV = 18,

    /// No such device
    #[error("[ENODEV] No such device")]
    ENODEV = 19,

    /// Not a directory
    #[error("[ENOTDIR] Not a directory")]
    ENOTDIR = 20,

    /// Is a directory
    #[error("[EISDIR] Is a directory")]
    EISDIR = 21,

    /// Invalid argument
    #[error("[EINVAL] Invalid argument")]
    EINVAL = 22,

    /// File table overflow
    #[error("[ENFILE] File table overflow")]
    ENFILE = 23,

    /// Too many open files
    #[error("[EMFILE] Too many open files")]
    EMFILE = 24,

    /// Not a typewriter
    #[error("[ENOTTY] Not a typewriter")]
    ENOTTY = 25,

    /// Text file busy
    #[error("[ETXTBSY] Text file busy")]
    ETXTBSY = 26,

    /// File too large
    #[error("[EFBIG] File too large")]
    EFBIG = 27,

    /// No space left on device
    #[error("[ENOSPC] No space left on device")]
    ENOSPC = 28,

    /// Illegal seek
    #[error("[ESPIPE] Illegal seek")]
    ESPIPE = 29,

    /// Read-only file system
    #[error("[EROFS] Read-only file system")]
    EROFS = 30,

    /// Too many links
    #[error("[EMLINK] Too many links")]
    EMLINK = 31,

    /// Broken pipe
    #[error("[EPIPE] Broken pipe")]
    EPIPE = 32,

    /// Math argument out of domain of func
    #[error("[EDOM] Math argument out of domain of func")]
    EDOM = 33,

    /// Math result not representable
    #[error("[ERANGE] Math result not representable")]
    ERANGE = 34,

    /// Connection timed out
    #[error("[ETIMEDOUT] Connection timed out")]
    ETIMEDOUT = 110,
}
