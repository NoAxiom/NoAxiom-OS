use alloc::{sync::Arc, vec::Vec};

use ksync::mutex::check_no_lock;

use super::Task;
use crate::{
    config::task::INIT_PROCESS_ID,
    cpu::current_cpu,
    mm::user_ptr::UserPtr,
    signal::{
        sig_detail::{SigChildDetail, SigDetail},
        sig_info::{SigCode, SigInfo},
        sig_num::SigNum,
    },
    syscall::Syscall,
    task::{
        manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
        status::TaskStatus,
    },
};

pub async fn init_proc_exit_handler(task: &Arc<Task>) {
    let inner = task.pcb();
    if !inner.children.is_empty() {
        warn!("[exit_handler] init_proc try to exited before its children!!!");
        let ch_tid: Vec<usize> = inner.children.iter().map(|it| it.tid()).collect();
        warn!("[exit_handler] child info: {:?}", ch_tid);
        while !inner.children.is_empty() {
            let pid = Syscall::new(task).sys_wait4(-1, 0, 0).await;
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
        _ => println!(
            "[kernel] init_proc exited unexpectedly, exit_code: {}",
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
        self.delete_children();
        self.thread_group().remove(tid);
        TASK_MANAGER.remove(tid);
        PROCESS_GROUP_MANAGER.lock().remove(self);

        // clear child tid
        if let Some(tidaddress) = self.clear_child_tid() {
            info!("[exit_handler] clear child tid {:#x}", tidaddress);
            let ptr = UserPtr::<usize>::new(tidaddress);
            assert!(check_no_lock());
            let _ = ptr.try_write(0).await;
            let _ = ptr
                .translate_pa()
                .await
                .map(|pa| self.futex().wake_waiter(pa, 1));
        }

        // send SIGCHLD to parent
        let mut pcb = self.pcb();
        pcb.set_status(TaskStatus::Zombie);
        if self.is_group_leader() {
            if let Some(process) = pcb.parent.clone() {
                let parent = process.upgrade().unwrap();
                trace!("[exit_handler] parent tid: {}", parent.tid());
                // send SIGCHLD
                let siginfo = SigInfo::new_detailed(
                    SigNum::SIGCHLD.into(),
                    SigCode::User,
                    0,
                    SigDetail::Child(SigChildDetail {
                        pid: self.tgid() as u32,
                        status: Some(pcb.exit_code()),
                        utime: None,
                        stime: None,
                    }),
                );
                parent.recv_siginfo(siginfo, false);
            } else {
                error!("[exit_handler] parent not found");
            }
        }
        warn!("[exit_hander] task {} exited successfully", self.tid());
        TASK_MANAGER.get_init_proc().print_child_tree();
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        let parent = self.pcb().parent.as_ref().unwrap().upgrade().unwrap();
        warn!(
            "[drop_task] task {} dropped, dropper: {}, parent {}",
            self.tid(),
            current_cpu()
                .task
                .as_ref()
                .map_or_else(|| 0, |task| task.tid()),
            parent.tid(),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExitCode(i32);
impl ExitCode {
    pub fn new(code: i32) -> Self {
        Self((code & 0xFF) << 8)
    }
    #[allow(unused)]
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
