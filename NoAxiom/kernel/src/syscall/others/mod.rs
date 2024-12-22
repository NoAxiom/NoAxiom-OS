use super::{nix::tms::TMS, Syscall};
use crate::sched::utils::yield_now;

impl Syscall<'_> {
    /// yield current task
    pub async fn sys_yield() -> isize {
        trace!("sys_yield");
        yield_now().await;
        0
    }

    pub fn sys_times(buf: *mut TMS) -> isize {
        -1
    }
}
