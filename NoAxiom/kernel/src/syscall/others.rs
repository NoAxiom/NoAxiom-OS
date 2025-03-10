use super::{Syscall, SyscallResult};
use crate::{
    include::{info::Utsname, time::TMS},
    mm::user_ptr::UserPtr,
    sched::utils::yield_now,
    time::{
        gettime::{get_time_ms, get_time_us, get_timeval},
        timeval::TimeVal,
    },
};

impl Syscall<'_> {
    /// yield current task
    pub async fn sys_yield(&self) -> SyscallResult {
        trace!("sys_yield");
        yield_now(self.task).await;
        Ok(0)
    }

    pub fn sys_times(tms: usize) -> SyscallResult {
        let tms = UserPtr::<TMS>::new(tms);
        let sec = get_time_us() as isize;
        let res = TMS {
            tms_utime: sec,
            tms_stime: sec,
            tms_cutime: sec,
            tms_cstime: sec,
        };
        tms.write_volatile(res);
        Ok(0)
    }

    pub fn sys_uname(buf: usize) -> SyscallResult {
        let buf = UserPtr::<Utsname>::new(buf);
        let res = Utsname::get();
        buf.write_volatile(res);
        Ok(0)
    }

    pub fn sys_gettimeofday(buf: usize) -> SyscallResult {
        if buf == 0 {
            return Ok(get_time_ms() as isize);
        }
        let buf = UserPtr::<TimeVal>::new(buf);
        let timeval = get_timeval();
        buf.write_volatile(timeval);
        Ok(0)
    }
}
