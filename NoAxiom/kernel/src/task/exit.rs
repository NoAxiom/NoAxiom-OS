use alloc::{sync::Arc, vec::Vec};

use arch::{Arch, ArchSbi};

use super::Task;
use crate::{
    config::task::INIT_PROCESS_ID,
    fs::vfs::root_dentry,
    signal::{
        sig_detail::{SigChildDetail, SigDetail},
        sig_info::{SigCode, SigInfo},
        sig_num::SigNum,
    },
    syscall::Syscall,
    task::{manager::TASK_MANAGER, status::TaskStatus},
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
    root_dentry().super_block().sync_all().await;
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
                debug!("parent tid: {}", parent.tid());

                // del self from parent's children, and wake up suspended parent
                let mut par_pcb = parent.pcb();
                self.set_status(TaskStatus::Zombie);
                par_pcb.children.retain(|task| task.tid() != tid);
                par_pcb.zombie_children.push(self.clone());
                // if par_pcb.wait_req {
                //     debug!("waking parent");
                //     par_pcb.wait_req = false;
                //     parent.wake();
                // } else {
                //     trace!("I suppose that my parent is already woken");
                // }

                // send SIGCHLD
                let siginfo = SigInfo {
                    signo: SigNum::SIGCHLD.into(),
                    code: SigCode::User,
                    errno: 0,
                    detail: SigDetail::Child(SigChildDetail {
                        pid: self.tgid() as u32,
                        status: Some(self.exit_code()),
                        utime: None,
                        stime: None,
                    }),
                };
                parent.recv_siginfo(siginfo, false);

                drop(par_pcb);
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
