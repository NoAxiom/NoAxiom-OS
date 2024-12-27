//! Linux error number: https://man7.org/linux/man-pages/man3/errno.3.html

// use alloc::boxed::Box;
// use core::{future::Future, pin::Pin};

use thiserror::Error;

// sync syscall result
// pub type GeneralRes<T> = core::result::Result<T, Errno>;

// async syscall result
// pub type SysFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
// pub type AsyscallRet<'a> = SysFuture<'a, Result>;

#[repr(i32)]
#[derive(Debug, Error, PartialEq, Eq, Clone)]
#[allow(dead_code, non_camel_case_types)]
pub enum Errno {
    /// Error cannot be clarified due to current implementation.
    ///
    /// # Note
    /// Should only be used for debugging.
    #[error("Error cannot be clarified due to current implementation")]
    DISCARD = -1,

    /// Operation not permitted.
    #[error("[EPERM] Operation not permitted")]
    EPERM = 1,

    /// No such file or directory.
    #[error("[ENOENT] No such file or directory")]
    ENOENT = 2,

    /// No such process.
    #[error("[ESRCH] No such process")]
    ESRCH = 3,

    /// Interrupted system call.
    #[error("[EINTR] Interrupted system call")]
    EINTR = 4,

    /// I/O error.
    #[error("[EIO] I/O error")]
    EIO = 5,

    /// No such device or address.
    #[error("[ENXIO] No such device or address")]
    ENXIO = 6,

    /// Argument list too long.
    #[error("[E2BIG] Argument list too long")]
    E2BIG = 7,

    /// Exec format error.
    #[error("[ENOEXEC] Exec format error")]
    ENOEXEC = 8,

    /// Bad file number.
    #[error("[EBADF] Bad file number")]
    EBADF = 9,

    /// No child processes.
    #[error("[ECHILD] No child processes")]
    ECHILD = 10,

    /// Try again.
    #[error("[EAGAIN] Try again")]
    EAGAIN = 11,

    /// Out of memory.
    #[error("[ENOMEM] Out of memory")]
    ENOMEM = 12,

    /// Permission denied.
    #[error("[EACCES] Permission denied1")]
    EACCES = 13,

    /// Bad address.
    #[error("[EFAULT] Bad address")]
    EFAULT = 14,

    /// Block device required.
    #[error("[ENOTBLK] Block device required")]
    ENOTBLK = 15,

    /// Device or resource busy.
    #[error("[EBUSY] Device or resource busy")]
    EBUSY = 16,

    /// File exists.
    #[error("[EEXIST] File exists")]
    EEXIST = 17,

    /// Cross-device link.
    #[error("[EXDEV] Cross-device link")]
    EXDEV = 18,

    /// No such device.
    #[error("[ENODEV] No such device")]
    ENODEV = 19,

    /// Not a directory.
    #[error("[ENOTDIR] Not a directory")]
    ENOTDIR = 20,

    /// Is a directory.
    #[error("[EISDIR] Is a directory")]
    EISDIR = 21,

    /// Invalid argument.
    #[error("[EINVAL] Invalid argument")]
    EINVAL = 22,

    /// File table overflow.
    #[error("[ENFILE] File table overflow")]
    ENFILE = 23,

    /// Too many open files.
    #[error("[EMFILE] Too many open files")]
    EMFILE = 24,

    /// Not a typewriter.
    #[error("[ENOTTY] Not a typewriter")]
    ENOTTY = 25,

    /// Text file busy.
    #[error("[ETXTBSY] Text file busy")]
    ETXTBSY = 26,

    /// File too large.
    #[error("[EFBIG] File too large")]
    EFBIG = 27,

    /// No space left on device.
    #[error("[ENOSPC] No space left on device")]
    ENOSPC = 28,

    /// Illegal seek.
    #[error("[ESPIPE] Illegal seek")]
    ESPIPE = 29,

    /// Read-only file system.
    #[error("[EROFS] Read-only file system")]
    EROFS = 30,

    /// Too many links.
    #[error("[EMLINK] Too many links")]
    EMLINK = 31,

    /// Broken pipe.
    #[error("[EPIPE] Broken pipe")]
    EPIPE = 32,

    /// Math argument out of domain of func.
    #[error("[EDOM] Math argument out of domain of func")]
    EDOM = 33,

    /// Math result not representable.
    #[error("[ERANGE] Math result not representable")]
    ERANGE = 34,

    /// Resource deadlock would occur.
    #[error("[EDEADLK_OR_EDEADLOCK] Resource deadlock would occur")]
    EDEADLK_OR_EDEADLOCK = 35,

    /// Filename too long.
    #[error("[ENAMETOOLONG] Filename too long")]
    ENAMETOOLONG = 36,

    /// No locks available.
    #[error("[ENOLCK] No locks available")]
    ENOLCK = 37,

    /// Function not supported.
    #[error("[ENOSYS] Function not supported")]
    ENOSYS = 38,

    /// Directory not empty.
    #[error("[ENOTEMPTY] Directory not empty")]
    ENOTEMPTY = 39,

    /// Too many levels of symbolic links.
    #[error("[ELOOP] Too many levels of symbolic links")]
    ELOOP = 40,

    /// No message of the desired type.
    #[error("[ENOMSG] No message of the desired type")]
    ENOMSG = 42,

    /// Identifier removed.
    #[error("[EIDRM] Identifier removed")]
    EIDRM = 43,

    /// Channel number out of range.
    #[error("[ECHRNG] Channel number out of range")]
    ECHRNG = 44,

    /// Level 2 not synchronized.
    #[error("[EL2NSYNC] Level 2 not synchronized")]
    EL2NSYNC = 45,

    /// Level 3 halted.
    #[error("[EL3HLT] Level 3 halted")]
    EL3HLT = 46,

    /// Level 3 reset.
    #[error("[EL3RST] Level 3 reset")]
    EL3RST = 47,

    /// Link number out of range.
    #[error("[ELNRNG] Link number out of range")]
    ELNRNG = 48,

    /// Protocol driver not attached.
    #[error("[EUNATCH] Protocol driver not attached")]
    EUNATCH = 49,

    /// No CSI structure available.
    #[error("[ENOCSI] No CSI structure available")]
    ENOCSI = 50,

    /// Level 2 halted.
    #[error("[EL2HLT] Level 2 halted")]
    EL2HLT = 51,

    /// Invalid exchange.
    #[error("[EBADE] Invalid exchange")]
    EBADE = 52,

    /// Invalid request descriptor.
    #[error("[EBADR] Invalid request descriptor")]
    EBADR = 53,

    /// Exchange full.
    #[error("[EXFULL] Exchange full")]
    EXFULL = 54,

    /// No anode.
    #[error("[ENOANO] No anode")]
    ENOANO = 55,

    /// Invalid request code.
    #[error("[EBADRQC] Invalid request code")]
    EBADRQC = 56,

    /// Invalid slot.
    #[error("[EBADSLT] Invalid slot")]
    EBADSLT = 57,

    /// Bad font file format.
    #[error("[EBFONT] Bad font file format")]
    EBFONT = 59,

    /// Not a STREAM.
    #[error("[ENOSTR] Not a STREAM")]
    ENOSTR = 60,

    /// No message is available on the STREAM head read queue.
    #[error("[ENODATA] No message is available on the STREAM head read queue")]
    ENODATA = 61,

    /// Stream ioctl() timeout.
    #[error("[ETIME] Stream ioctl() timeout")]
    ETIME = 62,

    /// No STREAM resources.
    #[error("[ENOSR] No STREAM resources")]
    ENOSR = 63,

    /// Machine is not on the network.
    #[error("[ENONET] Machine is not on the network")]
    ENONET = 64,

    /// Package not installed.
    #[error("[ENOPKG] Package not installed")]
    ENOPKG = 65,

    /// Object is remote.
    #[error("[EREMOTE] Object is remote")]
    EREMOTE = 66,

    /// Reserved.
    #[error("[ENOLINK] Reserved")]
    ENOLINK = 67,

    /// Advertise error.
    #[error("[EADV] Advertise error")]
    EADV = 68,

    /// Srmount error.
    #[error("[ESRMNT] Srmount error")]
    ESRMNT = 69,

    /// Communication error on send.
    #[error("[ECOMM] Communication error on send")]
    ECOMM = 70,

    /// Protocol error.
    #[error("[EPROTO] Protocol error")]
    EPROTO = 71,

    /// Reserved.
    #[error("[EMULTIHOP] Reserved")]
    EMULTIHOP = 72,

    /// RFS specific error.
    #[error("[EDOTDOT] RFS specific error")]
    EDOTDOT = 73,

    /// Bad message.
    #[error("[EBADMSG] Bad message")]
    EBADMSG = 74,

    /// Value too large to be stored in data type.
    #[error("[EOVERFLOW] Value too large to be stored in data type")]
    EOVERFLOW = 75,

    /// Name not unique on network.
    #[error("[ENOTUNIQ] Name not unique on network")]
    ENOTUNIQ = 76,

    /// File descriptor in bad state.
    #[error("[EBADFD] File descriptor in bad state")]
    EBADFD = 77,

    /// Remote address changed.
    #[error("[EREMCHG] Remote address changed")]
    EREMCHG = 78,

    /// Can not access a needed shared library.
    #[error("[ELIBACC] Can not access a needed shared library")]
    ELIBACC = 79,

    /// Accessing a corrupted shared library.
    #[error("[ELIBBAD] Accessing a corrupted shared library")]
    ELIBBAD = 80,

    /// .lib section in a.out corrupted.
    #[error("[ELIBSCN] .lib section in a.out corrupted")]
    ELIBSCN = 81,

    /// Attempting to link in too many shared libraries.
    #[error("[ELIBMAX] Attempting to link in too many shared libraries")]
    ELIBMAX = 82,

    /// Cannot exec a shared library directly.
    #[error("[ELIBEXEC] Cannot exec a shared library directly")]
    ELIBEXEC = 83,

    /// Illegal byte sequence.
    #[error("[EILSEQ] Illegal byte sequence")]
    EILSEQ = 84,

    /// Interrupted system call should be restarted.
    #[error("[ERESTART] Interrupted system call should be restarted")]
    ERESTART = 85,

    /// Streams pipe error.
    #[error("[ESTRPIPE] Streams pipe error")]
    ESTRPIPE = 86,

    /// Too many users.
    #[error("[EUSERS] Too many users")]
    EUSERS = 87,

    /// Socket operation on non-socket.
    #[error("[ENOTSOCK] Socket operation on non-socket")]
    ENOTSOCK = 88,

    /// Destination address required.
    #[error("[EDESTADDRREQ] Destination address required")]
    EDESTADDRREQ = 89,

    /// Message too long.
    #[error("[EMSGSIZE] Message too long")]
    EMSGSIZE = 90,

    /// Protocol wrong type for socket.
    #[error("[EPROTOTYPE] Protocol wrong type for socket")]
    EPROTOTYPE = 91,

    /// Protocol not available.
    #[error("[ENOPROTOOPT] Protocol not available")]
    ENOPROTOOPT = 92,

    /// Protocol not supported.
    #[error("[EPROTONOSUPPORT] Protocol not supported")]
    EPROTONOSUPPORT = 93,

    /// Socket type not supported.
    #[error("[ESOCKTNOSUPPORT] Socket type not supported")]
    ESOCKTNOSUPPORT = 94,

    /// Operation not supported on transport endpoint.
    #[error("[EOPNOTSUPP] Operation not supported on transport endpoint")]
    EOPNOTSUPP = 95,

    /// Protocol family not supported.
    #[error("[EPFNOSUPPORT] Protocol family not supported")]
    EPFNOSUPPORT = 96,

    /// Address family not supported by protocol.
    #[error("[EAFNOSUPPORT] Address family not supported by protocol")]
    EAFNOSUPPORT = 97,

    /// Address already in use.
    #[error("[EADDRINUSE] Address already in use")]
    EADDRINUSE = 98,

    /// Cannot assign requested address.
    #[error("[EADDRNOTAVAIL] Cannot assign requested address")]
    EADDRNOTAVAIL = 99,

    /// Network is down.
    #[error("[ENETDOWN] Network is down")]
    ENETDOWN = 100,

    /// Network is unreachable.
    #[error("[ENETUNREACH] Network is unreachable")]
    ENETUNREACH = 101,

    /// Network dropped connection because of reset.
    #[error("[ENETRESET] Network dropped connection because of reset")]
    ENETRESET = 102,

    /// Software caused connection abort.
    #[error("[ECONNABORTED] Software caused connection abort")]
    ECONNABORTED = 103,

    /// Connection reset by peer.
    #[error("[ECONNRESET] Connection reset by peer")]
    ECONNRESET = 104,

    /// No buffer space available.
    #[error("[ENOBUFS] No buffer space available")]
    ENOBUFS = 105,

    /// Transport endpoint is already connected.
    #[error("[EISCONN] Transport endpoint is already connected")]
    EISCONN = 106,

    /// Transport endpoint is not connected.
    #[error("[ENOTCONN] Transport endpoint is not connected")]
    ENOTCONN = 107,

    /// Cannot send after transport endpoint shutdown.
    #[error("[ESHUTDOWN] Cannot send after transport endpoint shutdown")]
    ESHUTDOWN = 108,

    /// Too many references: cannot splice.
    #[error("[ETOOMANYREFS] Too many references: cannot splice")]
    ETOOMANYREFS = 109,

    /// Connection timed out.
    #[error("[ETIMEDOUT] Connection timed out")]
    ETIMEDOUT = 110,

    /// Connection refused.
    #[error("[ECONNREFUSED] Connection refused")]
    ECONNREFUSED = 111,

    /// Host is down.
    #[error("[EHOSTDOWN] Host is down")]
    EHOSTDOWN = 112,

    /// No route to host.
    #[error("[EHOSTUNREACH] No route to host")]
    EHOSTUNREACH = 113,

    /// Operation already in progress.
    #[error("[EALREADY] Operation already in progress")]
    EALREADY = 114,

    /// Operation now in progress.
    #[error("[EINPROGRESS] Operation now in progress")]
    EINPROGRESS = 115,

    /// Stale NFS file handle.
    #[error("[ESTALE] Stale NFS file handle")]
    ESTALE = 116,

    /// Structure needs cleaning.
    #[error("[EUCLEAN] Structure needs cleaning")]
    EUCLEAN = 117,

    /// Not a XENIX named type file.
    #[error("[ENOTNAM] Not a XENIX named type file")]
    ENOTNAM = 118,

    /// No XENIX semaphores available.
    #[error("[ENAVAIL] No XENIX semaphores available")]
    ENAVAIL = 119,

    /// Is a named type file.
    #[error("[EISNAM] Is a named type file")]
    EISNAM = 120,

    /// Remote I/O error.
    #[error("[EREMOTEIO] Remote I/O error")]
    EREMOTEIO = 121,

    /// Quota exceeded.
    #[error("[EDQUOT] Quota exceeded")]
    EDQUOT = 122,

    /// No medium found.
    #[error("[ENOMEDIUM] No medium found")]
    ENOMEDIUM = 123,

    /// Wrong medium type.
    #[error("[EMEDIUMTYPE] Wrong medium type")]
    EMEDIUMTYPE = 124,

    /// Operation canceled.
    #[error("[ECANCELED] Operation canceled")]
    ECANCELED = 125,

    /// Required key not available.
    #[error("[ENOKEY] Required key not available")]
    ENOKEY = 126,

    /// Key has expired.
    #[error("[EKEYEXPIRED] Key has expired")]
    EKEYEXPIRED = 127,

    /// Key has been revoked.
    #[error("[EKEYREVOKED] Key has been revoked")]
    EKEYREVOKED = 128,

    /// Key was rejected by service.
    #[error("[EKEYREJECTED] Key was rejected by service")]
    EKEYREJECTED = 129,

    /// Owner died.
    #[error("[EOWNERDEAD] Owner died")]
    EOWNERDEAD = 130,

    /// State not recoverable.
    #[error("[ENOTRECOVERABLE] State not recoverable")]
    ENOTRECOVERABLE = 131,

    /// Operation not possible due to RF-kill.
    #[error("[ERFKILL] Operation not possible due to RF-kill")]
    ERFKILL = 132,

    /// Memory page has hardware error.
    #[error("[EHWPOISON] Memory page has hardware error")]
    EHWPOISON = 133,
}
