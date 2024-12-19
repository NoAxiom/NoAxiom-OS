pub struct TMS {
    /// 用户态时间
    pub tms_utime: isize,
    /// 内核态时间
    pub tms_stime: isize,
    /// 已回收子进程的用户态时间
    pub tms_cutime: isize,
    /// 已回收子进程的内核态时间
    pub tms_cstime: isize,
}