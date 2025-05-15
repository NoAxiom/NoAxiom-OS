use include::errno::Errno;

use super::{Syscall, SyscallResult};
use crate::{
    include::sched::{CpuMask, SchedParam, SCHED_OTHER},
    mm::user_ptr::UserPtr,
    sched::utils::yield_now,
    task::manager::TASK_MANAGER,
};

impl Syscall<'_> {
    /// yield current task
    pub async fn sys_yield(&self) -> SyscallResult {
        trace!("sys_yield");
        yield_now().await;
        Ok(0)
    }

    pub fn sys_sched_getaffinity(
        &self,
        pid: usize,
        cpusetsize: usize,
        mask: usize,
    ) -> SyscallResult {
        let mask = UserPtr::<CpuMask>::new(mask);
        let task_mask = if let Some(task) = TASK_MANAGER.get(pid) {
            let tg = task.thread_group();
            if let Some(Some(task)) = tg.0.get(&pid).map(|t| t.upgrade()) {
                task.tcb().cpu_mask
            } else {
                return Err(Errno::ESRCH);
            }
        } else {
            return Err(Errno::ESRCH);
        };
        mask.write(task_mask);
        Ok(0)
    }

    pub fn sys_sched_setaffinity(
        &self,
        pid: usize,
        cpusetsize: usize,
        mask: usize,
    ) -> SyscallResult {
        let mask = UserPtr::<CpuMask>::new(mask).read();
        if let Some(task) = TASK_MANAGER.get(pid) {
            let tg = task.thread_group();
            if let Some(Some(task)) = tg.0.get(&pid).map(|t| t.upgrade()) {
                task.tcb_mut().cpu_mask = mask
            } else {
                return Err(Errno::ESRCH);
            }
        } else {
            return Err(Errno::ESRCH);
        };
        Ok(0)
    }

    pub fn sys_sched_setscheduler(
        &self,
        pid: usize,
        policy: isize,
        param: usize, // ptr
    ) -> SyscallResult {
        Ok(0)
    }

    pub fn sys_sched_getscheduler(&self, pid: usize) -> SyscallResult {
        Ok(SCHED_OTHER)
    }

    pub fn sys_sched_getparam(&self, pid: usize, param: usize) -> SyscallResult {
        let param = UserPtr::<SchedParam>::new(param);
        let user_param = param.as_ref_mut()?;
        user_param.set_priority(1);
        Ok(0)
    }
}
