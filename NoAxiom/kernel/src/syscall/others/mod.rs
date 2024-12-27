use super::{Syscall, SyscallResult};
use crate::{nix::tms::TMS, sched::utils::yield_now};

impl Syscall<'_> {
    /// yield current task
    pub async fn sys_yield() -> SyscallResult {
        trace!("sys_yield");
        yield_now().await;
        Ok(0)
    }

    pub fn sys_times(tms: usize) -> SyscallResult {
        let tms = tms as *mut TMS;
        todo!()
    }
}
