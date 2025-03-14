use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::Ordering,
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
            PidSel::Task(Some(pid)) => match pcb.find_child(pid) {
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
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        let mut pcb = self.task.pcb();
        let res = match &self.target {
            None => match pcb.zombie_children.pop() {
                Some(task) => Poll::Ready(Ok((task.exit_code(), task.tid()))),
                None => Poll::Pending,
            },
            Some(task) => match task.status() {
                TaskStatus::Zombie => {
                    let target_tid = task.tid();
                    let exit_code = task.exit_code();
                    pcb.zombie_children.retain(|task| task.tid() != target_tid);
                    Poll::Ready(Ok((exit_code, target_tid)))
                }
                _ => Poll::Pending,
            },
        };
        match res {
            Poll::Pending => {
                if self.wait_option.contains(WaitOption::WNOHANG) && res.is_pending() {
                    Poll::Ready(Ok((0, 0)))
                } else {
                    trace!("[sys_wait4] suspend for child exit");
                    pcb.wait_req.store(true, Ordering::Release);
                    Poll::Pending
                }
            }
            Poll::Ready(_) => res,
        }
    }
}

/*

垃圾rust, 只有match写出来的代码才漂亮, 搞一坨if搞出来的难看死了
想把代码写漂亮真难啊

*/
