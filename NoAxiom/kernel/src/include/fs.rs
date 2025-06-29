use bitflags::bitflags;
use config::fs::BLOCK_SIZE;
use strum::FromRepr;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct FileFlags: u32 {
        // TODO do not use 0
        // NOTE: bitflags do not encourage zero bit flag, we should not directly check `O_RDONLY`
        const O_RDONLY    = 0;
        const O_WRONLY    = 1 << 0;
        const O_RDWR      = 1 << 1;
        const O_CREATE    = 1 << 6;
        const O_EXCL      = 1 << 7;
        const O_TRUNC     = 1 << 9;
        const O_APPEND    = 1 << 10;
        const O_NONBLOCK  = 1 << 11;
        const O_LARGEFILE = 1 << 15;
        const O_DIRECTORY = 1 << 16;
        const O_NOFOLLOW  = 1 << 17;
        const O_CLOEXEC   = 1 << 19;
    }

    /// User group read and write permissions
    #[derive(Debug)]
    pub struct CreateMode: u32 {
        const S_ISUID  = 0o4000;
        const S_ISGID  = 0o2000;
        const S_ISVTX  = 0o1000;

        const S_IRWXU  = 0o700;
        const S_IRUSR  = 0o400;
        const S_IWUSR  = 0o200;
        const S_IXUSR  = 0o100;

        const S_IRWXG  = 0o070;
        const S_IRGRP  = 0o040;
        const S_IWGRP  = 0o020;
        const S_IXGRP  = 0o010;

        const S_IRWXO  = 0o007;
        const S_IROTH  = 0o004;
        const S_IWOTH  = 0o002;
        const S_IXOTH  = 0o001;
    }

    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub struct InodeMode: u32 {
        /// Type.
        const TYPE_MASK = 0o170000;
        /// FIFO.
        const FIFO  = 0o010000;
        /// Character device.
        const CHAR  = 0o020000;
        /// Directory
        const DIR   = 0o040000;
        /// Block device
        const BLOCK = 0o060000;
        /// Regular file.
        const FILE  = 0o100000;
        /// Symbolic link.
        const LINK  = 0o120000;
        /// Socket
        const SOCKET = 0o140000;

        /// Set-user-ID on execution.
        const SET_UID = 0o4000;
        /// Set-group-ID on execution.
        const SET_GID = 0o2000;
        /// sticky bit
        const STICKY = 0o1000;
        /// Read, write, execute/search by owner.
        const OWNER_MASK = 0o700;
        /// Read permission, owner.
        const OWNER_READ = 0o400;
        /// Write permission, owner.
        const OWNER_WRITE = 0o200;
        /// Execute/search permission, owner.
        const OWNER_EXEC = 0o100;

        /// Read, write, execute/search by group.
        const GROUP_MASK = 0o70;
        /// Read permission, group.
        const GROUP_READ = 0o40;
        /// Write permission, group.
        const GROUP_WRITE = 0o20;
        /// Execute/search permission, group.
        const GROUP_EXEC = 0o10;

        /// Read, write, execute/search by others.
        const OTHER_MASK = 0o7;
        /// Read permission, others.
        const OTHER_READ = 0o4;
        /// Write permission, others.
        const OTHER_WRITE = 0o2;
        /// Execute/search permission, others.
        const OTHER_EXEC = 0o1;
    }

    #[derive(Debug)]
    pub struct MountFlags:u32 {
        /// This filesystem is mounted read-only.
        const MS_RDONLY = 1;
        /// The set-user-ID and set-group-ID bits are ignored by exec(3) for executable files on this filesystem.
        const MS_NOSUID = 1 << 1;
        /// Disallow access to device special files on this filesystem.
        const MS_NODEV = 1 << 2;
        /// Execution of programs is disallowed on this filesystem.
        const MS_NOEXEC = 1 << 3;
        /// Writes are synched to the filesystem immediately (see the description of O_SYNC in open(2)).
        const MS_SYNCHRONOUS = 1 << 4;
        /// Alter flags of a mounted FS
        const MS_REMOUNT = 1 << 5;
        /// Allow mandatory locks on an FS
        const MS_MANDLOCK = 1 << 6;
        /// Directory modifications are synchronous
        const MS_DIRSYNC = 1 << 7;
        /// Do not follow symlinks
        const MS_NOSYMFOLLOW = 1 << 8;
        /// Do not update access times.
        const MS_NOATIME = 1 << 10;
        /// Do not update directory access times
        const MS_NODEIRATIME = 1 << 11;
        const MS_BIND = 1 << 12;
        const MS_MOVE = 1 << 13;
        const MS_REC = 1 << 14;
        /// War is peace. Verbosity is silence.
        const MS_SILENT = 1 << 15;
        /// VFS does not apply the umask
        const MS_POSIXACL = 1 << 16;
        /// change to unbindable
        const MS_UNBINDABLE = 1 << 17;
        /// change to private
        const MS_PRIVATE = 1 << 18;
        /// change to slave
        const MS_SLAVE = 1 << 19;
        /// change to shared
        const MS_SHARED = 1 << 20;
        /// Update atime relative to mtime/ctime.
        const MS_RELATIME = 1 << 21;
        /// this is a kern_mount call
        const MS_KERNMOUNT = 1 << 22;
        /// Update inode I_version field
        const MS_I_VERSION = 1 << 23;
        /// Always perform atime updates
        const MS_STRICTATIME = 1 << 24;
        /// Update the on-disk [acm]times lazily
        const MS_LAZYTIME = 1 << 25;
        /// These sb flags are internal to the kernel
        const MS_SUBMOUNT = 1 << 26;
        const MS_NOREMOTELOCK = 1 << 27;
        const MS_NOSEC = 1 << 28;
        const MS_BORN = 1 << 29;
        const MS_ACTIVE = 1 << 30;
        const MS_NOUSER = 1 << 31;
    }
}

impl From<CreateMode> for InodeMode {
    fn from(mode: CreateMode) -> Self {
        InodeMode::from_bits_truncate(mode.bits())
    }
}

impl From<InodeMode> for CreateMode {
    fn from(mode: InodeMode) -> Self {
        CreateMode::from_bits_truncate(mode.bits())
    }
}

#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad: u64,
    pub st_size: u64,
    pub st_blksize: u32,
    pub __pad2: u32,
    pub st_blocks: u64,
    pub st_atime_sec: u64,
    pub st_atime_nsec: u64,
    pub st_mtime_sec: u64,
    pub st_mtime_nsec: u64,
    pub st_ctime_sec: u64,
    pub st_ctime_nsec: u64,
    pub unused: u64,
} // 128

#[derive(Clone, Copy, Debug)]
#[repr(C)]
/// Store the file attributes from a supported file.
pub struct Statx {
    pub stx_mask: u32,
    pub stx_blksize: u32,
    pub stx_attributes: u64,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: u16,
    __statx_pad1: [u16; 1],
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: StatxTimestamp,
    pub stx_btime: StatxTimestamp,
    pub stx_ctime: StatxTimestamp,
    pub stx_mtime: StatxTimestamp,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub stx_mnt_id: u64,
    __statx_pad2: u64,
    __statx_pad3: [u64; 12],
}
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StatxTimestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __statx_timestamp_pad1: [i32; 1],
}

impl StatxTimestamp {
    pub fn new(tv_sec: i64, tv_nsec: u32) -> Self {
        Self {
            tv_sec,
            tv_nsec,
            __statx_timestamp_pad1: [0; 1],
        }
    }
}

impl Statx {
    #![allow(unused)]
    /// Get the inode number described in the `Stat`
    pub fn get_ino(&self) -> usize {
        self.stx_ino as usize
    }
    pub fn get_size(&self) -> usize {
        self.stx_size as usize
    }
    pub fn new(
        stx_mask: u32,
        stx_nlink: u32,
        stx_mode: u16,
        stx_ino: u64,
        stx_size: u64,
        stx_atime: StatxTimestamp,
        stx_ctime: StatxTimestamp,
        stx_mtime: StatxTimestamp,
        stx_rdev_major: u32,
        stx_rdev_minor: u32,
        stx_dev_major: u32,
        stx_dev_minor: u32,
    ) -> Self {
        Self {
            stx_mask,
            stx_blksize: BLOCK_SIZE as u32,
            stx_attributes: 0,
            stx_nlink,
            stx_uid: 0,
            stx_gid: 0,
            stx_mode,
            __statx_pad1: [0 as u16; 1],
            stx_ino,
            stx_size,
            stx_blocks: (stx_size as u64 + BLOCK_SIZE as u64 - 1) / BLOCK_SIZE as u64,
            stx_attributes_mask: 0,
            stx_atime,
            stx_btime: stx_ctime,
            stx_ctime,
            stx_mtime,
            stx_rdev_major,
            stx_rdev_minor,
            stx_dev_major,
            stx_dev_minor,
            stx_mnt_id: 0,
            __statx_pad2: 0,
            __statx_pad3: [0 as u64; 12],
        }
    }
}

// Defined in <bits/struct_stat.h>
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Kstat {
    /// 设备
    pub st_dev: u64,
    /// inode 编号
    pub st_ino: u64,
    /// 文件类型
    pub st_mode: u32,
    /// 硬链接数
    pub st_nlink: u32,
    /// 用户 id
    pub st_uid: u32,
    /// 用户组 id
    pub st_gid: u32,
    /// 设备号
    pub st_rdev: u64,
    _pad0: u64,
    /// 文件大小
    pub st_size: i64,
    /// 块大小
    pub st_blksize: i32,
    _pad1: i32,
    /// 块个数
    pub st_blocks: i64,
    /// 最后一次访问时间 (秒)
    pub st_atime_sec: isize,
    /// 最后一次访问时间 (纳秒)
    pub st_atime_nsec: isize,
    /// 最后一次修改时间 (秒)
    pub st_mtime_sec: isize,
    /// 最后一次修改时间 (纳秒)
    pub st_mtime_nsec: isize,
    /// 最后一次改变状态时间 (秒)
    pub st_ctime_sec: isize,
    /// 最后一次改变状态时间 (纳秒)
    pub st_ctime_nsec: isize,
    __unused: [u32; 2],
}

impl Kstat {
    pub fn from_stat(stat: Stat) -> Self {
        Kstat {
            st_dev: stat.st_dev,
            st_ino: stat.st_ino,
            st_mode: stat.st_mode, // 0777 permission, we don't care about permission
            st_nlink: stat.st_nlink,
            st_uid: stat.st_uid,
            st_gid: stat.st_gid,
            st_rdev: stat.st_rdev,
            _pad0: stat.__pad,
            st_size: stat.st_size as i64,
            st_blksize: stat.st_blksize as i32,
            _pad1: stat.__pad2 as i32,
            st_blocks: stat.st_blocks as i64,
            st_atime_sec: stat.st_atime_sec as isize,
            st_atime_nsec: stat.st_atime_nsec as isize,
            st_mtime_sec: stat.st_mtime_sec as isize,
            st_mtime_nsec: stat.st_mtime_nsec as isize,
            st_ctime_sec: stat.st_ctime_sec as isize,
            st_ctime_nsec: stat.st_ctime_nsec as isize,
            __unused: [0; 2],
        }
    }
}

#[derive(Debug, Clone, Default)]
#[allow(unused)]
pub struct LinuxDirent64 {
    d_ino: u64,
    d_off: u64,
    d_reclen: u16,
    d_type: u8,
    // d_name follows here, which will be written later
}

impl LinuxDirent64 {
    pub fn new(d_ino: u64, d_off: u64, d_reclen: u16, d_type: u8) -> Self {
        Self {
            d_ino,
            d_off,
            d_reclen,
            d_type,
        }
    }
}

bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct Ext4DirEntryType: u8 {
        const EXT4_DE_UNKNOWN = 0;
        const EXT4_DE_REG_FILE = 1;
        const EXT4_DE_DIR = 2;
        const EXT4_DE_CHRDEV = 3;
        const EXT4_DE_BLKDEV = 4;
        const EXT4_DE_FIFO = 5;
        const EXT4_DE_SOCK = 6;
        const EXT4_DE_SYMLINK = 7;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Iovec {
    pub iov_base: usize,
    pub iov_len: usize,
}

impl Iovec {
    pub const fn size() -> usize {
        core::mem::size_of::<Iovec>()
    }
}

bitflags! {
    #[derive(PartialEq, Eq, Debug)]
    pub struct FcntlFlags: usize {
        const F_DUPFD = 0;
        const F_GETFD = 1;
        const F_SETFD = 2;
        const F_GETFL = 3;
        const F_SETFL = 4;
        const F_GETLK = 5;
        const F_SETLK = 6;
        const F_SETLKW = 7;
        const F_SETOWN = 8;
        const F_GETOWN = 9;
        const F_SETSIG = 10;
        const F_GETSIG = 11;
        const F_SETOWN_EX = 15;
        const F_GETOWN_EX = 16;
        const F_GETOWNER_UIDS = 17;

        // 发现 F_UNLCK = 2 , 这个标记分类待研究
        const F_DUPFD_CLOEXEC = 1030;
    }

    #[derive(PartialEq, Eq, Debug, Clone)]
    pub struct FcntlArgFlags: u32 {
        const FD_CLOEXEC = 1;
        const AT_EMPTY_PATH = 1 << 0;
        const AT_SYMLINK_NOFOLLOW = 1 << 8;
        const AT_EACCESS = 1 << 9;
        const AT_NO_AUTOMOUNT = 1 << 11;
        const AT_DUMMY = 1 << 12;
    }
}

impl FcntlArgFlags {
    pub fn from_arg(arg: FileFlags) -> Self {
        if arg.contains(FileFlags::O_CLOEXEC) {
            FcntlArgFlags::FD_CLOEXEC
        } else {
            FcntlArgFlags::empty()
        }
    }
}

// for /proc fs
pub const TOTAL_MEM: usize = 16251136;
pub const FREE_MEM: usize = 327680;
pub const BUFFER: usize = 373336;
pub const CACHED: usize = 10391984;
pub const TOTAL_SWAP: usize = 4194300;

/// Mapping to free output: https://access.redhat.com/solutions/406773.
pub struct MemInfo {
    /// General memory
    pub total_mem: usize,
    pub free_mem: usize,
    pub avail_mem: usize,
    /// Buffer and cache
    pub buffers: usize,
    pub cached: usize,
    /// Swap space
    pub total_swap: usize,
    pub free_swap: usize,
    /// Share memory
    pub shmem: usize,
    pub slab: usize,
}

impl MemInfo {
    pub const fn new() -> Self {
        Self {
            total_mem: TOTAL_MEM,
            free_mem: FREE_MEM,
            avail_mem: TOTAL_MEM - FREE_MEM,
            buffers: BUFFER,
            cached: CACHED,
            total_swap: TOTAL_SWAP,
            free_swap: TOTAL_SWAP,
            shmem: 0,
            slab: 0,
        }
    }
    pub fn serialize(&self) -> alloc::string::String {
        use alloc::string::ToString;
        let mut res = "".to_string();
        let end = " KB\n";
        let total_mem = "MemTotal:\t".to_string() + self.total_mem.to_string().as_str() + end;
        let free_mem = "MemFree:\t".to_string() + self.free_mem.to_string().as_str() + end;
        let avail_mem = "MemAvailable:\t".to_string() + self.avail_mem.to_string().as_str() + end;
        let buffers = "Buffers:\t".to_string() + self.buffers.to_string().as_str() + end;
        let cached = "Cached:\t".to_string() + self.cached.to_string().as_str() + end;
        let cached_swap = "SwapCached:\t".to_string() + 0.to_string().as_str() + end;
        let total_swap = "SwapTotal:\t".to_string() + self.total_swap.to_string().as_str() + end;
        let free_swap = "SwapFree:\t".to_string() + self.free_swap.to_string().as_str() + end;
        let shmem = "Shmem:\t".to_string() + self.shmem.to_string().as_str() + end;
        let slab = "Slab:\t".to_string() + self.slab.to_string().as_str() + end;
        res += total_mem.as_str();
        res += free_mem.as_str();
        res += avail_mem.as_str();
        res += buffers.as_str();
        res += cached.as_str();
        res += cached_swap.as_str();
        res += total_swap.as_str();
        res += free_swap.as_str();
        res += shmem.as_str();
        res += slab.as_str();
        res
    }
}

#[derive(Default)]
#[repr(C)]
pub struct RtcTime {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
}

/// Defined in <asm-generic/ioctls.h>
#[derive(FromRepr, Debug)]
#[repr(usize)]
pub enum TtyIoctlCmd {
    // For struct termios
    /// Gets the current serial port settings.
    TCGETS = 0x5401,
    /// Sets the serial port settings immediately.
    TCSETS = 0x5402,
    /// Sets the serial port settings after allowing the input and output
    /// buffers to drain/empty.
    TCSETSW = 0x5403,
    /// Sets the serial port settings after flushing the input and output
    /// buffers.
    TCSETSF = 0x5404,
    /// For struct termio
    /// Gets the current serial port settings.
    TCGETA = 0x5405,
    /// Sets the serial port settings immediately.
    #[allow(unused)]
    TCSETA = 0x5406,
    /// Sets the serial port settings after allowing the input and output
    /// buffers to drain/empty.
    #[allow(unused)]
    TCSETAW = 0x5407,
    /// Sets the serial port settings after flushing the input and output
    /// buffers.
    #[allow(unused)]
    TCSETAF = 0x5408,
    /// If the terminal is using asynchronous serial data transmission, and arg
    /// is zero, then send a break (a stream of zero bits) for between 0.25
    /// and 0.5 seconds.
    TCSBRK = 0x5409,
    /// Get the process group ID of the foreground process group on this
    /// terminal.
    TIOCGPGRP = 0x540F,
    /// Set the foreground process group ID of this terminal.
    TIOCSPGRP = 0x5410,
    /// Get window size.
    TIOCGWINSZ = 0x5413,
    /// Set window size.
    TIOCSWINSZ = 0x5414,
}

#[derive(FromRepr, Debug)]
#[repr(usize)]
pub enum RtcIoctlCmd {
    /// Get the current RTC time at usize.
    RTCRDTIME = 0xffffffff80247009,
}

#[derive(FromRepr, Debug)]
#[repr(usize)]
pub enum NoAxiomIoctlCmd {
    /// ONLY FOR TESTS
    INTOTESTCASE = 0x114514,
    OUTOFTESTCASE = 0x1919810,
}

#[derive(Debug)]
#[repr(usize)]
pub enum IoctlCmd {
    Tty(TtyIoctlCmd),
    Rtc(RtcIoctlCmd),
    Other(NoAxiomIoctlCmd),
}

/// Defined in <asm-generic/termbits.h>
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Termios {
    /// Input mode flags.
    pub iflag: u32,
    /// Output mode flags.
    pub oflag: u32,
    /// Control mode flags.
    pub cflag: u32,
    /// Local mode flags.
    pub lflag: u32,
    /// Line discipline.
    pub line: u8,
    /// control characters.
    pub cc: [u8; 19],
}

impl Termios {
    pub fn new() -> Self {
        Self {
            // IMAXBEL | IUTF8 | IXON | IXANY | ICRNL | BRKINT
            iflag: 0o66402,
            // OPOST | ONLCR
            oflag: 0o5,
            // HUPCL | CREAD | CSIZE | EXTB
            cflag: 0o2277,
            // IEXTEN | ECHOTCL | ECHOKE ECHO | ECHOE | ECHOK | ISIG | ICANON
            lflag: 0o105073,
            line: 0,
            cc: [
                3,   // VINTR Ctrl-C
                28,  // VQUIT
                127, // VERASE
                21,  // VKILL
                4,   // VEOF Ctrl-D
                0,   // VTIME
                1,   // VMIN
                0,   // VSWTC
                17,  // VSTART
                19,  // VSTOP
                26,  // VSUSP Ctrl-Z
                255, // VEOL
                18,  // VREPAINT
                15,  // VDISCARD
                23,  // VWERASE
                22,  // VLNEXT
                255, // VEOL2
                0, 0,
            ],
        }
    }

    #[allow(unused)]
    pub fn is_icrnl(&self) -> bool {
        const ICRNL: u32 = 0o0000400;
        self.iflag & ICRNL != 0
    }

    #[allow(unused)]
    pub fn is_echo(&self) -> bool {
        const ECHO: u32 = 0o0000010;
        self.lflag & ECHO != 0
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WinSize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16, // Unused
    ws_ypixel: u16, // Unused
}

impl WinSize {
    pub fn new() -> Self {
        Self {
            ws_row: 67,
            ws_col: 120,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}

/// Enumeration of possible methods to seek within an I/O object.
///
/// It is based on the `std::io::SeekFrom` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    /// Sets the offset to the provided number of bytes.
    Start(u64),
    /// Sets the offset to the size of this object plus the specified number of
    /// bytes.
    End(i64),
    /// Sets the offset to the current position plus the specified number of
    /// bytes.
    Current(i64),
}

#[derive(FromRepr, Debug)]
#[repr(usize)]
pub enum Whence {
    SeekSet = 0,
    SeekCur = 1,
    SeekEnd = 2,
    SeekData = 3,
    SeekHold = 4,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    // Defined in <stdio.h>.
    pub struct RenameFlags: i32 {
        /// Don't overwrite newpath of the rename. Return an error if newpath already exists.
        const RENAME_NOREPLACE = 1 << 0;
        /// Atomically exchange oldpath and newpath.
        const RENAME_EXCHANGE = 1 << 1;
        /// This operation makes sense only for overlay/union filesystem implementations.
        const RENAME_WHITEOUT = 1 << 2;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Statfs {
    pub f_type: u64,
    pub f_bsize: u64,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: u64,
    pub f_namelen: u64,
    pub f_frsize: u64,
    pub f_flag: u64,
    pub f_spare: [u64; 4],
}
impl Statfs {
    // TODO
    pub fn new() -> Self {
        Self {
            f_type: 1,
            f_bsize: 512,
            f_blocks: 12345,
            f_bfree: 1234,
            f_bavail: 123,
            f_files: 1000,
            f_ffree: 100,
            f_fsid: 1,
            f_namelen: 123,
            f_frsize: 4096,
            f_flag: 123,
            f_spare: [0; 4],
        }
    }
    #[allow(unused)]
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}
