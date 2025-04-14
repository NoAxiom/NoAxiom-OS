#[repr(C)]
pub struct TMS {
    /// user time
    pub tms_utime: usize,
    /// system time
    pub tms_stime: usize,
    /// user time of dead children
    pub tms_cutime: usize,
    /// system time of dead children
    pub tms_cstime: usize,
}
