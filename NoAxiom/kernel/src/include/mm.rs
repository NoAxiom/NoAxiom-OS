use bitflags::bitflags;

use crate::mm::pte::PTEFlags;

bitflags! {
#[derive(Clone, Copy, Debug)]
    /// see [man mmap](https://man7.org/linux/man-pages/man2/mmap.2.html)
    /// The prot argument describes the desired memory protection of the
    /// mapping (and must not conflict with the open mode of the file).
    /// It is either PROT_NONE or the bitwise OR of one or more of the
    /// following flags:
    pub struct MmapProts: usize {
        const PROT_NONE = 0;       // Pages may not be accessed. Used for guard pages.
        const PROT_READ = 1 << 0;  // Pages may be read.
        const PROT_WRITE = 1 << 1; // Pages may be written.
        const PROT_EXEC  = 1 << 2; // Pages may be executed.
        const PROT_GROWSDOWN = 0x01000000;
        const PROT_GROWSUP = 0x02000000;
    }
}

impl From<MmapProts> for PTEFlags {
    fn from(prots: MmapProts) -> Self {
        let mut flags = PTEFlags::empty();
        if prots.contains(MmapProts::PROT_READ) {
            flags |= PTEFlags::R;
        }
        if prots.contains(MmapProts::PROT_WRITE) {
            flags |= PTEFlags::W;
        }
        if prots.contains(MmapProts::PROT_EXEC) {
            flags |= PTEFlags::X;
        }
        flags
    }
}

bitflags! {
#[derive(Clone, Copy, Debug)]
    pub struct MmapFlags: usize {
        /// File mapping. Used for file initialization.(for compatibility, can be ignored)
        const MAP_FILE = 0;

        /// Process shared, changes to the mapping are visible to other processes mapping the same region
        const MAP_SHARED = 0x01;

        /// Process private, copy-on-write. Need to set the prot of parent and child process to read-only.
        /// This causes a page fault exception when writing, and then process it.
        const MAP_PRIVATE = 0x02;

        /// Map the mmap space to the memory address specified by addr.
        /// If it overlaps with an existing mapped page, the overlapping part is discarded.
        /// If the specified address cannot be used, mmap will fail.
        const MAP_FIXED = 0x10;

        /// Anonymous mapping, initialize the memory space that is all 0.
        /// When fd is -1 and MAP_ANONYMOUS flag exists, mmap will create an anonymous mapping
        const MAP_ANONYMOUS = 0x20;
    }
}

// pub struct SharedMemoryIdentifierDs {
//     pub shm_perm: CreateMode, /* Ownership and permissions */
//     pub shm_size: usize,      /* Size of segment (bytes) */
//     pub shm_atime: usize,     /* Last attach time */
//     pub shm_dtime: usize,     /* Last detach time */
//     pub shm_ctime: usize,     /* Creation time/time of last modification via
// shmctl() */     pub shm_cpid: usize,      /* PID of creator */
//     pub shm_lpid: usize,      /* PID of last shmat(2)/shmdt(2) */
//     pub shm_nattch: usize,    /* Number of current attaches */
// }
