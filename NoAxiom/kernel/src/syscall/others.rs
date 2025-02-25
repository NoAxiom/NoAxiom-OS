use super::{Syscall, SyscallResult};
use crate::{mm::user_ptr::UserPtr, sched::utils::yield_now, time::gettime::get_time_us};

impl Syscall<'_> {
    /// yield current task
    pub async fn sys_yield() -> SyscallResult {
        trace!("sys_yield");
        yield_now().await;
        Ok(0)
    }

    pub fn sys_times(tms: usize) -> SyscallResult {
        #[allow(unused)]
        struct TMS {
            /// user time
            tms_utime: isize,
            /// system time
            tms_stime: isize,
            /// user time of dead children
            tms_cutime: isize,
            /// system time of dead children
            tms_cstime: isize,
        }
        let tms = UserPtr::<TMS>::new(tms);
        let sec = get_time_us() as isize;
        let res = TMS {
            tms_utime: sec,
            tms_stime: sec,
            tms_cutime: sec,
            tms_cstime: sec,
        };
        unsafe { tms.set(res) };
        Ok(0)
    }
}
