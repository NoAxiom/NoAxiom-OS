pub const AT_FDCWD: isize = -100;

// pub const RLIMIT_CPU: u32 = 0; // CPU 使用时间的最大量
// pub const RLIMIT_FSIZE: u32 = 1; // 可以创建的文件的最大大小
// pub const RLIMIT_DATA: u32 = 2; // 程序的数据段的最大字节长度
// pub const RLIMIT_STACK: u32 = 3; // 栈的最大字节长度
// pub const RLIMIT_CORE: u32 = 4; // 核心文件的最大字节长度
// pub const RLIMIT_RSS: u32 = 5; // 最大驻留集大小
// pub const RLIMIT_NPROC: u32 = 6; // 用户可以拥有的最大进程数
// pub const RLIMIT_NOFILE: u32 = 7; // 用户可以打开的最大文件数
// pub const RLIMIT_MEMLOCK: u32 = 8; // 用户可以锁定的最大内存量
// pub const RLIMIT_AS: u32 = 9; // 进程的最大地址空间字节长度
// pub const RLIMIT_LOCKS: u32 = 10; // 用户可以拥有的最大文件锁数
// pub const RLIMIT_SIGPENDING: u32 = 11; // 用户可以排队的最大信号数
// pub const RLIMIT_MSGQUEUE: u32 = 12; // 用户可以为 POSIX
// 消息队列分配的最大字节长度 pub const RLIMIT_NICE: u32 = 13; //
// 用户可以设置的最大 nice 值 pub const RLIMIT_RTPRIO: u32 = 14; //
// 用户可以设置的最大实时优先级 pub const RLIMIT_RTTIME: u32 = 15; //
// 用户可以消耗的最大实时 CPU 时间 pub const RLIMIT_NLIMITS: u32 = 16; //
// 限制的总数

pub const MAX_FD_COUNT: usize = 1024;
pub const RLIMIT_SOFT_MAX: usize = MAX_FD_COUNT;
pub const RLIMIT_HARD_MAX: usize = usize::MAX; // unlimited

pub const LEN_BEFORE_NAME: usize = 19;

pub const UTIME_NOW: usize = 0x3fffffff;
pub const UTIME_OMIT: usize = 0x3ffffffe;

pub const F_OK: i32 = 0;
pub const X_OK: i32 = 1;
pub const W_OK: i32 = 2;
pub const R_OK: i32 = 4;
pub const UID_ROOT: u32 = 0;
pub const AT_EACCESS: i32 = 0x200;

/// 请求文件类型 (stx_mode & S_IFMT)
pub const STATX_TYPE: u32 = 0x00000001;
/// 请求文件权限模式 (stx_mode & ~S_IFMT)
pub const STATX_MODE: u32 = 0x00000002;
/// 请求硬链接计数 (stx_nlink)
pub const STATX_NLINK: u32 = 0x00000004;
/// 请求文件所有者UID (stx_uid)
pub const STATX_UID: u32 = 0x00000008;
/// 请求文件所属组GID (stx_gid)
pub const STATX_GID: u32 = 0x00000010;
/// 请求最后访问时间 (stx_atime)
pub const STATX_ATIME: u32 = 0x00000020;
/// 请求最后修改时间 (stx_mtime)
pub const STATX_MTIME: u32 = 0x00000040;
/// 请求最后状态变更时间 (stx_ctime)
pub const STATX_CTIME: u32 = 0x00000080;
/// 请求inode编号 (stx_ino)
pub const STATX_INO: u32 = 0x00000100;
/// 请求文件大小 (stx_size)
pub const STATX_SIZE: u32 = 0x00000200;
/// 请求分配的磁盘块数 (stx_blocks)
pub const STATX_BLOCKS: u32 = 0x00000400;

/// 基础统计信息 (包含传统stat结构中的所有字段)
pub const STATX_BASIC_STATS: u32 = 0x000007ff;
pub const STATX_ALL: u32 = 0x00000fff;

/// 请求文件创建时间 (stx_btime)
pub const STATX_BTIME: u32 = 0x00000800;
/// 请求挂载点ID (stx_mnt_id)
pub const STATX_MNT_ID: u32 = 0x00001000;
/// 请求直接I/O对齐信息
pub const STATX_DIOALIGN: u32 = 0x00002000;
/// 请求扩展的唯一挂载点ID
pub const STATX_MNT_ID_UNIQUE: u32 = 0x00004000;
/// 请求子卷信息 (stx_subvol)
pub const STATX_SUBVOL: u32 = 0x00008000;
