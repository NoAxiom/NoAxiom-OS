use super::time_info::TimeInfo;

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

impl From<TimeInfo> for TMS {
    fn from(time_stat: TimeInfo) -> Self {
        let time = time_stat.time();
        let ctime = time_stat.child_time();
        Self {
            tms_utime: time.utime.as_micros() as usize,
            tms_stime: time.stime.as_micros() as usize,
            tms_cutime: ctime.utime.as_micros() as usize,
            tms_cstime: ctime.stime.as_micros() as usize,
        }
    }
}
