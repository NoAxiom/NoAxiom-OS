pub const STD_IN: usize = 0;
pub const STD_OUT: usize = 1;
pub const STD_ERR: usize = 2;

pub const AT_FDCWD: isize = -100;

pub const RLIMIT_CPU: u32 = 0; // CPU 使用时间的最大量
pub const RLIMIT_FSIZE: u32 = 1; // 可以创建的文件的最大大小
pub const RLIMIT_DATA: u32 = 2; // 程序的数据段的最大字节长度
pub const RLIMIT_STACK: u32 = 3; // 栈的最大字节长度
pub const RLIMIT_CORE: u32 = 4; // 核心文件的最大字节长度
pub const RLIMIT_RSS: u32 = 5; // 最大驻留集大小
pub const RLIMIT_NPROC: u32 = 6; // 用户可以拥有的最大进程数
pub const RLIMIT_NOFILE: u32 = 7; // 用户可以打开的最大文件数
pub const RLIMIT_MEMLOCK: u32 = 8; // 用户可以锁定的最大内存量
pub const RLIMIT_AS: u32 = 9; // 进程的最大地址空间字节长度
pub const RLIMIT_LOCKS: u32 = 10; // 用户可以拥有的最大文件锁数
pub const RLIMIT_SIGPENDING: u32 = 11; // 用户可以排队的最大信号数
pub const RLIMIT_MSGQUEUE: u32 = 12; // 用户可以为 POSIX 消息队列分配的最大字节长度
pub const RLIMIT_NICE: u32 = 13; // 用户可以设置的最大 nice 值
pub const RLIMIT_RTPRIO: u32 = 14; // 用户可以设置的最大实时优先级
pub const RLIMIT_RTTIME: u32 = 15; // 用户可以消耗的最大实时 CPU 时间
pub const RLIMIT_NLIMITS: u32 = 16; // 限制的总数

pub const MAX_FD_COUNT: usize = 1024;
pub const RLIMIT_SOFT_MAX: usize = MAX_FD_COUNT;
pub const RLIMIT_HARD_MAX: usize = usize::MAX; // unlimited

pub const LEN_BEFORE_NAME: usize = 19;
