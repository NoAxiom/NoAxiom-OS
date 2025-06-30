use alloc::sync::Arc;

use ksync::assert_no_lock;

use super::Task;
use crate::{
    config::task::INIT_PROCESS_ID,
    cpu::current_cpu,
    include::futex::FUTEX_BITSET_MATCH_ANY,
    mm::user_ptr::UserPtr,
    signal::{
        sig_detail::{SigChildDetail, SigDetail},
        sig_info::{SigCode, SigInfo},
        sig_num::SigNum,
    },
    task::{
        manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
        status::TaskStatus,
    },
};

pub async fn init_proc_exit_handler(task: &Arc<Task>) {
    task.fd_table().exit_files();
    let inner = task.pcb();
    if !inner.children.is_empty() {
        error!("[exit_handler] ERROR: init_proc try to exited before its children!!!");
        for i in inner.children.iter() {
            error!(
                "[exit_handler] child tid: {}, during syscall: {:?}, sigmask: {:?}",
                i.tid(),
                i.tcb().current_syscall,
                i.pcb().sig_mask(),
            );
            i.recv_siginfo(
                SigInfo {
                    signo: SigNum::SIGKILL.into(),
                    code: SigCode::Kernel,
                    errno: 0,
                    detail: SigDetail::None,
                },
                true,
            );
        }
        error!("[exit_handler] forced shutdown");
        // let mut cnt = 0;
        // while !inner.children.is_empty() {
        //     let pid = task
        //         .wait_child(PidSel::Task(None), WaitOption::WNOHANG)
        //         .await;
        //     yield_now().await;
        //     if let Ok(pid) = pid {
        //         info!("[exit_handler] child finally exited: {:?}", pid);
        //     }
        //     cnt += 1;
        //     if cnt > 10000000 {
        //         error!("[exit_handler] init_proc exit handler timeout, force
        // shutdown");         break;
        //     }
        // }
    }
    let exit_code = inner.exit_code();
    // !PAY ATTENTION!! Now we don't sync_all the dirty data.
    // root_dentry().super_block().sync_all().await;
    match exit_code.inner() {
        0 => info!(
            "[exit_handler] init_proc exited successfully, exit_code: {}",
            exit_code.inner()
        ),
        _ => println!(
            "[kernel] init_proc exited unexpectedly, exit_code: {}",
            exit_code.inner(),
        ),
    }
    println!("[kernel] system shutdown (normal exit)");
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
        self.fd_table().exit_files();
        self.delete_children();
        self.thread_group().remove(tid);
        TASK_MANAGER.remove(tid);
        PROCESS_GROUP_MANAGER.lock().remove(self);

        // clear child tid
        if let Some(tidaddress) = self.tcb().clear_child_tid {
            info!("[exit_handler] clear child tid {:#x}", tidaddress);
            let ptr = UserPtr::<usize>::new(tidaddress);
            assert_no_lock!();
            let _ = ptr.try_write(0).await;
            let _ = ptr
                .translate_pa()
                .await
                .inspect_err(|err| error!("[exit_handler] clear child tid failed: {}", err))
                .map(|pa| self.futex().wake_waiter(pa, 1, FUTEX_BITSET_MATCH_ANY));
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
                        status: Some(pcb.exit_code().inner()),
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
        // TASK_MANAGER.get_init_proc().print_child_tree();
        // let tids = TASK_MANAGER
        //     .0
        //     .lock()
        //     .iter()
        //     .map(|(tid, _)| *tid)
        //     .collect::<Vec<_>>();
        // println!("[exit_handler] all tasks: {:?}", tids);
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        // let parent = self.pcb().parent.as_ref().unwrap().upgrade().unwrap();
        warn!(
            "[drop_task] task {} dropped, dropper: {}",
            self.tid(),
            current_cpu()
                .task
                .as_ref()
                .map_or_else(|| 0, |task| task.tid()),
            // parent.tid(),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExitReason(i32); // exit code, signo
impl ExitReason {
    pub fn new(code: i32, signo: i32) -> Self {
        Self(((code & 0xFF) << 8) + (signo & 0x7f))
    }
    pub fn to_raw(self) -> i32 {
        (self.0 >> 8) & 0xFF
    }
    pub fn inner(&self) -> i32 {
        self.0
    }
}
impl Default for ExitReason {
    fn default() -> Self {
        Self(0)
    }
}
