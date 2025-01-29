use super::{Syscall, SyscallResult};
use crate::{
    mm::user_ptr::UserPtr, nix::tms::TMS, sched::utils::yield_now, time::gettime::get_time_us,
};

impl Syscall<'_> {
    /// yield current task
    pub async fn sys_yield() -> SyscallResult {
        debug!("sys_yield");
        yield_now().await;
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
        tms.set(res);
        Ok(0)
    }
}
