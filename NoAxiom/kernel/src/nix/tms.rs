pub struct TMS {
    /// user time
    pub tms_utime: isize,
    /// system time
    pub tms_stime: isize,
    /// user time of dead children
    pub tms_cutime: isize,
    /// system time of dead children
    pub tms_cstime: isize,
}