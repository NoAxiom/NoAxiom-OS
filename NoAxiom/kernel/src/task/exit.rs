use alloc::sync::Arc;

use ksync::assert_no_lock;

use super::Task;
use crate::{
    config::task::INIT_PROCESS_ID,
    cpu::current_cpu,
    include::futex::FUTEX_BITSET_MATCH_ANY,
    mm::user_ptr::UserPtr,
    panic::kshutdown,
    signal::{
        sig_detail::{SigChildDetail, SigDetail},
        sig_info::{SigCode, SigInfo},
        signal::Signal,
    },
    task::{
        futex::FUTEX_SHARED_QUEUE,
        manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
        status::TaskStatus,
    },
};

pub async fn init_proc_exit_handler(task: &Arc<Task>) {
    task.fd_table().exit_files();
    let pcb = task.pcb();
    if !pcb.children.is_empty() {
        warn!("[exit_handler] init_proc is trying to exited before its children!!!");
        for child in pcb.children.iter() {
            let child_pcb = child.pcb();
            warn!(
                "[exit_handler] child tid: {}, during syscall: {:?}, pending: {} sigmask: {:?}",
                child.tid(),
                child.tcb().current_syscall,
                child_pcb.signals.pending_set.debug_info_short(),
                child.sig_mask(),
            );
        }
    }
    let exit_code = pcb.exit_code();
    // !PAY ATTENTION!! Now we don't sync_all the dirty data.
    // root_dentry().super_block().sync_all().await;
    match exit_code.inner() {
        0 => info!(
            "[exit_handler] init_proc exited successfully, exit_code: {}",
            exit_code.inner()
        ),
        _ => warn!(
            "[kernel] init_proc exited unexpectedly, exit_code: {}",
            exit_code.inner(),
        ),
    }
    println!("[kernel] system shutdown (normal exit)");
    drop(pcb);
    kshutdown();
}

impl Task {
    pub async fn exit_handler(self: &Arc<Self>) {
        let tid = self.tid();
        if tid == INIT_PROCESS_ID {
            init_proc_exit_handler(self).await;
            unreachable!()
        }

        // thread resources clean up
        self.put_fd_table();
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
            let _ = self
                .futex()
                .wake_waiter(ptr.va_addr(), 1, FUTEX_BITSET_MATCH_ANY);
            let _ = ptr
                .translate_pa()
                .await
                .inspect_err(|err| error!("[exit_handler] clear child tid failed: {}", err))
                .map(|pa| {
                    FUTEX_SHARED_QUEUE
                        .lock()
                        .wake_waiter(pa, 1, FUTEX_BITSET_MATCH_ANY)
                });
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

        // handle vfork and wake parent if flag is detected
        self.vfork_callback();
        warn!("[exit_hander] task {} exited successfully", self.tid());
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
        );
        Self::del_dir_proc(self.tid());
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExitCode(i32); // exit code, signo
impl ExitCode {
    pub fn new(code: i32) -> Self {
        Self((code & 0xFF) << 8)
    }
    /// set the signal number in the exit code
    pub fn signaled(self, signal: Signal) -> Self {
        Self((self.0 & 0xFF00) | (signal as i32 & 0x7f))
    }
    /// indicate that the process was dumped
    pub fn core_dumped(self) -> Self {
        Self(self.0 | 0x80)
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
