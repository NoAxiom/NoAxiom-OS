use alloc::{sync::Arc, vec::Vec};

use super::Task;
use crate::{
    config::task::INIT_PROCESS_ID,
    mm::user_ptr::UserPtr,
    signal::{
        sig_detail::{SigChildDetail, SigDetail},
        sig_info::{SigCode, SigInfo},
        sig_num::SigNum,
    },
    syscall::Syscall,
    task::{manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER}, status::TaskStatus},
};

pub async fn init_proc_exit_handler(task: &Arc<Task>) {
    let inner = task.pcb();
    if !inner.children.is_empty() {
        warn!("[exit_handler] init_proc try to exited before its children!!!");
        let ch_tid: Vec<usize> = inner.children.iter().map(|it| it.tid()).collect();
        warn!("[exit_handler] child info: {:?}", ch_tid);
        while !inner.children.is_empty() {
            let pid = Syscall::new(task).sys_wait4(-1, 0, 0, 0).await;
            if let Ok(pid) = pid {
                info!("[exit_handler] child finally exited: {:?}", pid);
            }
        }
    }
    let exit_code = inner.exit_code();
    // !PAY ATTENTION!! Now we don't sync_all the dirty data.
    // root_dentry().super_block().sync_all().await;
    match exit_code {
        0 => info!(
            "[exit_handler] init_proc exited successfully, exit_code: {}",
            exit_code
        ),
        _ => panic!(
            "[exit_handler] init_proc exited unexpectedly, exit_code: {}",
            exit_code,
        ),
    }
    platform::shutdown();
}

impl Task {
    pub async fn exit_handler(self: &Arc<Self>) {
        let tid = self.tid();
        if tid == INIT_PROCESS_ID {
            init_proc_exit_handler(self).await;
            unreachable!()
        }

        // thread resources clean up
        self.thread_group().remove(tid);
        TASK_MANAGER.remove(tid);
        PROCESS_GROUP_MANAGER.remove(self);
        self.delete_children();

        // clear child tid
        if let Some(tidaddress) = self.clear_child_tid() {
            info!(
                "[exit_handler] clear child tid {:#x}, (unimpl futex)",
                tidaddress
            );
            let ptr = UserPtr::<u8>::new(tidaddress);
            let res = ptr
                .as_slice_mut_checked_raw(core::mem::size_of::<usize>())
                .await
                .unwrap();
            res[0] = 0;
            // FIXME: IMPL THIS AFTER FUTEX
            // task.futex_queue.lock().wake(tidaddress as u32, 1);
        }

        // send SIGCHLD to parent
        if self.is_group_leader() {
            let mut pcb = self.pcb();
            if let Some(process) = pcb.parent.clone() {
                let parent = process.upgrade().unwrap();
                trace!("[exit_handler] parent tid: {}", parent.tid());

                // del self from parent's children, and wake up suspended parent
                let mut par_pcb = parent.pcb();
                pcb.set_status(TaskStatus::Zombie);
                par_pcb.children.retain(|task| task.tid() != tid);
                par_pcb.zombie_children.push(self.clone());

                // send SIGCHLD
                let siginfo = SigInfo {
                    signo: SigNum::SIGCHLD.into(),
                    code: SigCode::User,
                    errno: 0,
                    detail: SigDetail::Child(SigChildDetail {
                        pid: self.tgid() as u32,
                        status: Some(pcb.exit_code()),
                        utime: None,
                        stime: None,
                    }),
                };
                parent.recv_siginfo(&mut par_pcb, siginfo, false);
                drop(par_pcb);
            } else {
                error!("[exit_handler] parent not found");
            }
        }
        info!("[exit_hander] task {} exited successfully", self.tid());
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        info!("task {} dropped", self.tid())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExitCode(i32);
impl ExitCode {
    pub fn new(code: i32) -> Self {
        Self((code & 0xFF) << 8)
    }
    pub fn new_raw(code: i32) -> Self {
        Self(code)
    }
    pub fn inner(&self) -> i32 {
        self.0
    }
}
impl Default for ExitCode {
    fn default() -> Self {
        Self(0)
    }
}
