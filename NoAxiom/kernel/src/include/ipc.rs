use bitflags::bitflags;
use strum::FromRepr;

use crate::include::fs::CreateMode;

/// The name choice IPC_PRIVATE was perhaps unfortunate, IPC_NEW would
/// more clearly show its function.
pub const IPC_PRIVATE: usize = 0;
pub const IPC_NEW: usize = IPC_PRIVATE;

pub const SHM_MIN: usize = 1;
pub const SHM_MAX: usize = 4096;

pub struct IpcPerm {
    pub __key: usize,             /* Key */
    pub uid: u32,                 /* Owner's user ID */
    pub gid: u32,                 /* Owner's group ID */
    pub cuid: u32,                /* Creator's user ID */
    pub cgid: u32,                /* Creator's group ID */
    pub mode: CreateMode,         /* Read/write permission */
    pub __seq: u16,               /* Sequence number */
    pub __pad2: u16,              /* Padding for alignment */
    pub __glibc_reserved1: usize, /* Reserved for future use */
    pub __glibc_reserved2: usize, /* Reserved for future use */
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct IpcGetFlags: usize {
        /// Create key if it does not exist
        const IPC_CREAT = 00001000;
        /// Fail if key exists
        const IPC_EXCL = 00002000;
        /// Return error on wait
        const IPC_NOWAIT = 00004000;
    }
}

#[repr(usize)]
#[allow(non_camel_case_types)]
#[derive(FromRepr, PartialEq, Eq, Debug, Clone, Copy)]
pub enum IpcCtlCmd {
    IPC_RMID = 0,
    IPC_SET = 1,
    IPC_STAT = 2,
    IPC_INFO = 3,
}

/// Data structure describing a shared memory segment
pub struct ShmIdDs {
    pub shm_perm: IpcPerm, /* Ownership and permissions */
    pub shm_size: usize,   /* Size of segment (bytes) */
    pub shm_atime: usize,  /* Last attach time */
    pub shm_dtime: usize,  /* Last detach time */
    pub shm_ctime: usize,  /* Creation time/time of last modification via shmctl() */
    pub shm_cpid: usize,   /* PID of creator */
    pub shm_lpid: usize,   /* PID of last shmat(2)/shmdt(2) */
    pub shm_nattch: usize, /* Number of current attaches */
}
