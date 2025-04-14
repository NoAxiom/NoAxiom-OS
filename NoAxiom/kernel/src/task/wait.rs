use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::{status::TaskStatus, Task};
use crate::{
    include::{
        process::{PidSel, WaitOption},
        result::Errno,
    },
    sched::utils::{after_suspend, before_suspend},
    signal::sig_set::SigSet,
    syscall::SysResult,
};

pub struct WaitChildFuture {
    task: Arc<Task>,
    target: Option<Arc<Task>>,
    wait_option: WaitOption,
}

impl WaitChildFuture {
    pub fn new(task: Arc<Task>, pid_type: PidSel, wait_option: WaitOption) -> SysResult<Self> {
        let pcb = task.pcb();
        let target = match pid_type {
            PidSel::Task(None) => None,
            PidSel::Task(Some(tgid)) => match pcb.children.iter().find(|task| task.tid() == tgid) {
                Some(task) => Some(task.clone()),
                None => return Err(Errno::ECHILD),
            },
            PidSel::Group(_) => return Err(Errno::EINVAL),
        };
        drop(pcb);
        Ok(Self {
            task,
            target,
            wait_option,
        })
    }
}

impl Future for WaitChildFuture {
    type Output = SysResult<(i32, usize)>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pcb = self.task.pcb();
        let res = match &self.target {
            None => match pcb.zombie_children.pop() {
                Some(child) => {
                    // time statistic
                    self.task
                        .tcb_mut()
                        .time_stat
                        .add_child_time(child.tcb().time_stat.child_time());
                    Poll::Ready(Ok((pcb.exit_code(), child.tid())))
                }
                None => Poll::Pending,
            },
            Some(child) => {
                let mut ch_pcb = child.pcb();
                match ch_pcb.status() {
                    TaskStatus::Zombie => {
                        let target_tid = child.tid();
                        let exit_code = ch_pcb.exit_code();
                        // since we already collected exit info
                        // so just delete it from zombie children
                        ch_pcb
                            .zombie_children
                            .retain(|task| task.tid() != target_tid);
                        // update time statistic
                        self.task
                            .tcb_mut()
                            .time_stat
                            .add_child_time(child.tcb().time_stat.child_time());
                        Poll::Ready(Ok((exit_code, target_tid)))
                    }
                    _ => Poll::Pending,
                }
            }
        };
        let res = match res {
            Poll::Pending => {
                if self.wait_option.contains(WaitOption::WNOHANG) && res.is_pending() {
                    trace!("[sys_wait4] return nohang");
                    Poll::Ready(Ok((0, 0)))
                } else {
                    trace!("[sys_wait4] suspend for child exit");
                    Poll::Pending
                }
            }
            Poll::Ready(_) => {
                trace!("[sys_wait4] exited child found");
                res
            }
        };
        match res {
            Poll::Pending => {
                before_suspend(pcb, Some(SigSet::SIGCHLD));
            }
            Poll::Ready(_) => {
                after_suspend(Some(pcb));
            }
        }
        res
    }
}
