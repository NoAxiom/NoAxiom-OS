use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::Ordering;

use arch::{Arch, ArchSbi};

use super::Task;
use crate::{
    config::task::INIT_PROCESS_ID,
    include::signal::{
        sig_info::{SigCode, SigExtraInfo, SigInfo},
        sig_num::SigNum,
    },
    syscall::Syscall,
    task::manager::TASK_MANAGER,
};

pub async fn init_proc_exit_handler(task: &Arc<Task>) {
    if task.pcb().children.is_empty() {
        info!(
            "[exit_handler] init_proc exited successfully, exit_code: {}",
            task.exit_code()
        );
    } else {
        warn!("[exit_handler] init_proc try to exited before its children!!!");
        let ch_tid: Vec<usize> = task.pcb().children.iter().map(|it| it.tid()).collect();
        warn!("[exit_handler] child info: {:?}", ch_tid);
        while !task.pcb().children.is_empty() {
            let pid = Syscall::new(task).sys_wait4(-1, 0, 0, 0).await;
            if let Ok(pid) = pid {
                info!("[exit_handler] child finally exited: {:?}", pid);
            }
        }
        info!(
            "[exit_handler] init_proc exited successfully, exit_code: {}",
            task.exit_code()
        );
    }
    Arch::shutdown();
}

impl Task {
    pub async fn exit_handler(self: &Arc<Self>) {
        let tid = self.tid();
        if tid == INIT_PROCESS_ID {
            init_proc_exit_handler(self).await;
            unreachable!()
        }

        // thread resources clean up
        self.thread_group.lock().remove(tid);
        TASK_MANAGER.remove(tid);
        self.delete_children();

        // send SIGCHLD to parent
        if self.is_group_leader() {
            let parent = self.pcb().parent.clone();
            if let Some(process) = parent {
                let parent = process.upgrade().unwrap();

                // send SIGCHLD
                let siginfo = SigInfo {
                    signo: SigNum::SIGCHLD.into(),
                    code: SigCode::User,
                    errno: 0,
                    extra_info: SigExtraInfo::Extend {
                        si_pid: self.tgid() as u32,
                        si_status: Some(self.exit_code()),
                        si_utime: None,
                        si_stime: None,
                    },
                };
                parent.proc_recv_siginfo(siginfo);

                // del self from parent's children, and wake up suspended parent
                let mut par_pcb = parent.pcb();
                par_pcb.children.retain(|task| task.tid() != tid);
                par_pcb.zombie_children.push(self.clone());
                if par_pcb.wait_req.load(Ordering::Acquire) {
                    debug!("waking parent");
                    par_pcb.wait_req.store(false, Ordering::Release);
                    parent.wake_up();
                } else {
                    debug!("I suppose that my parent is already woken");
                }
            }
        }

        info!(
            "[exit_hander] task {} exited successfully, exit_code: {}, strong_count: {}",
            self.tid(),
            self.exit_code(),
            Arc::strong_count(self),
        );
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        info!(
            "task {} dropped, exit_code: {}",
            self.tid(),
            self.exit_code()
        )
    }
}
