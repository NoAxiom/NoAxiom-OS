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
    syscall::SysResult,
};

pub struct WaitChildFuture<'a> {
    task: &'a Arc<Task>,
    target: Option<Arc<Task>>,
    wait_option: WaitOption,
}

impl<'a> WaitChildFuture<'a> {
    pub fn new(task: &'a Arc<Task>, pid_type: PidSel, wait_option: WaitOption) -> SysResult<Self> {
        let pcb = task.pcb();
        if pcb.children.is_empty() {
            return Err(Errno::ECHILD);
        }
        let target = match pid_type {
            PidSel::Task(None) => None,
            PidSel::Task(Some(tgid)) => match pcb.children.iter().find(|child| child.tid() == tgid)
            {
                Some(child) => Some(child.clone()),
                None => return Err(Errno::ECHILD),
            },
            PidSel::Group(_) => return Err(Errno::EINVAL),
        };
        debug!("wait for child: {:?}", target.as_ref().map(|x| x.tid()));
        drop(pcb);
        Ok(Self {
            task,
            target,
            wait_option,
        })
    }
}

impl Future for WaitChildFuture<'_> {
    type Output = SysResult<(i32, usize)>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let res = match &self.target {
            None => {
                let mut pcb = self.task.pcb();
                match pcb.pop_one_zombie_child() {
                    Some(child) => {
                        // time statistic
                        let child_tid = child.tid();
                        self.task
                            .time_stat_mut()
                            .add_child_time(child.time_stat().child_time());
                        Poll::Ready(Ok((pcb.exit_code(), child_tid)))
                    }
                    None => Poll::Pending,
                }
            }
            Some(child) => {
                let ch_pcb = child.pcb();
                match ch_pcb.status() {
                    TaskStatus::Zombie => {
                        let child_tid = child.tid();
                        let exit_code = ch_pcb.exit_code();
                        drop(ch_pcb);
                        // since we already collected exit info
                        // so just delete it from zombie children
                        let mut pcb = self.task.pcb();
                        // remove child from parent
                        pcb.children.retain(|task| task.tid() != child_tid);
                        // update time statistic
                        self.task
                            .time_stat_mut()
                            .add_child_time(child.time_stat().child_time());
                        Poll::Ready(Ok((exit_code, child_tid)))
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
        res
    }
}

impl Task {
    pub async fn wait_child(
        self: &Arc<Self>,
        pid_type: PidSel,
        wait_option: WaitOption,
    ) -> SysResult<(i32, usize)> {
        WaitChildFuture::new(self, pid_type, wait_option)?.await
    }
}

/*

use alloc::{sync::Arc, vec::Vec};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use arch::TrapArgs;

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

pub struct WaitChildFuture<'a> {
    task: &'a Arc<Task>,
    pid_type: PidSel,
    wait_option: WaitOption,
}

impl Future for WaitChildFuture<'_> {
    type Output = SysResult<(i32, usize)>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pcb = self.task.pcb();
        let res = match self.pid_type {
            PidSel::Task(None) => {
                if pcb.children.is_empty() && pcb.zombie_children.is_empty() {
                    // empty child: return ECHILD
                    return Poll::Ready(Err(Errno::ECHILD));
                } else {
                    // try to find a zombie child
                    let child_tids: Vec<_> = pcb.children.iter().map(|x| x.tid()).collect();
                    let child_pcs: Vec<_> = pcb
                        .children
                        .iter()
                        .map(|x| x.trap_context()[TrapArgs::EPC])
                        .collect();
                    let zombie_tids = pcb
                        .zombie_children
                        .iter()
                        .map(|x| x.tid())
                        .collect::<Vec<_>>();
                    warn!(
                        "[sys_wait4] wait all, children: {:?}, pc: {:x?}, zombie: {:?}",
                        child_tids, child_pcs, zombie_tids,
                    );
                    match pcb.zombie_children.pop() {
                        Some(child) => {
                            // time statistic
                            self.task
                                .tcb_mut()
                                .time_stat
                                .add_child_time(child.tcb().time_stat.child_time());
                            Poll::Ready(Ok((pcb.exit_code(), child.tid())))
                        }
                        None => Poll::Pending,
                    }
                }
            }
            PidSel::Task(Some(tgid)) => match pcb.children.iter().find(|task| task.tid() == tgid) {
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
                None => return Poll::Ready(Err(Errno::ECHILD)),
            },
            PidSel::Group(_) => return Poll::Ready(Err(Errno::EINVAL)),
        };
        let res = match res {
            Poll::Pending => {
                if self.wait_option.contains(WaitOption::WNOHANG) {
                    warn!("[sys_wait4] return nohang");
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

impl Task {
    pub async fn wait_child(
        self: &Arc<Self>,
        pid_type: PidSel,
        wait_option: WaitOption,
    ) -> SysResult<(i32, usize)> {
        WaitChildFuture {
            task: self,
            pid_type,
            wait_option,
        }
        .await
    }
}

*/
