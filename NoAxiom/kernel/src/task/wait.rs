use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::{exit::ExitCode, status::TaskStatus, Task};
use crate::{
    include::{
        process::{PidSel, WaitOption},
        result::Errno,
    },
    return_errno,
    syscall::SysResult,
};

type WaitChildOutput = (ExitCode, usize);

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
                None => return_errno!(Errno::ECHILD),
            },
            PidSel::Group(_) => {
                error!("WARN: using unimpl Group wait pid type in WaitChildFuture");
                //return_errno!(Errno::EINVAL, "pid_type: {:?}", pid_type),
                None
            }
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
    type Output = SysResult<WaitChildOutput>;
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
                        let exit_code = ch_pcb.exit_code;
                        debug!(
                            "[wait4] child_tid: {}, exit_code: {}",
                            child_tid,
                            exit_code.inner()
                        );
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
                if self.wait_option.contains(WaitOption::WNOHANG) {
                    // Poll::Ready(Err(Errno::EAGAIN))
                    Poll::Ready(Ok((ExitCode::default(), 0usize)))
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(_) => res,
        };
        res
    }
}

impl Task {
    pub async fn wait_child(
        self: &Arc<Self>,
        pid_type: PidSel,
        wait_option: WaitOption,
    ) -> SysResult<WaitChildOutput> {
        WaitChildFuture::new(self, pid_type, wait_option)?.await
    }
}
