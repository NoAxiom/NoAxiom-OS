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
        signal::Signal,
    },
    task::{
        manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
        status::TaskStatus,
    },
    utils::hack::is_ltp,
};

pub async fn init_proc_exit_handler(task: &Arc<Task>) {
    task.fd_table().exit_files();
    let pcb = task.pcb();
    if !pcb.children.is_empty() {
        warn!("[exit_handler] init_proc is trying to exited before its children!!!");
        // for child in pcb.children.iter() {
        //     warn!(
        //         "[exit_handler] child tid: {}, during syscall: {:?}, sigmask:
        // {:?}",         child.tid(),
        //         child.tcb().current_syscall,
        //         child.sig_mask(),
        //     );
        //     child.recv_siginfo(
        //         SigInfo {
        //             signal: Signal::SIGKILL,
        //             code: SigCode::Kernel,
        //             errno: -1,
        //             detail: SigDetail::None,
        //         },
        //         true,
        //     );
        //     child.wake_unchecked();
        // }
        // drop(pcb);
        // warn!("[exit_handler] trying to wait for children to exit...");
        // let mut cnt = 0;
        // let begin_time = get_time_duration();
        // loop {
        //     let pid = task
        //         .wait_child(PidSel::Task(None), WaitOption::WNOHANG)
        //         .await;
        //     if task.pcb().children.is_empty() {
        //         warn!("[exit_handler] all children exited");
        //         break;
        //     }
        //     yield_now().await;
        //     if let Ok(pid) = pid {
        //         warn!("[exit_handler] child got forced exit: {:?}", pid);
        //     }
        //     cnt += 1;
        //     if cnt > 10000
        //         || get_time_duration().saturating_sub(begin_time) > Duration::from_secs(5)
        //     {
        //         error!("[exit_handler] init_proc exit handler timeout, forced
        // shutdown");         break;
        //     }
        // }
    }
    let exit_code = pcb.exit_code();
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
        pcb.set_status(TaskStatus::Zombie, self.tif_mut());
        if self.is_group_leader() {
            if let Some(process) = pcb.parent.as_ref() {
                let parent = process.upgrade().unwrap();
                trace!("[exit_handler] parent tid: {}", parent.tid());
                // send SIGCHLD
                let siginfo = SigInfo::new_detailed(
                    Signal::SIGCHLD,
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
    pub fn new(code: i32, signo: usize) -> Self {
        let signo = signo as i32;
        if !is_ltp() {
            Self((code & 0xFF) << 8)
        } else {
            Self(((code & 0xFF) << 8) + (signo & 0x7f))
        }
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
