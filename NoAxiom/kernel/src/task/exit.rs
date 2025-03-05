use alloc::{sync::Arc, vec::Vec};

use super::Task;
use crate::{
    config::task::INIT_PROCESS_ID,
    include::signal::{
        sig_info::{SigCode, SigExtraInfo, SigInfo},
        sig_num::SigNum,
    },
    task::manager::TASK_MANAGER,
};

pub fn terminate_all_tasks() {
    todo!()
}

pub fn exit_handler(task: &Arc<Task>) {
    let tid = task.tid();
    let exit_code = task.exit_code();
    trace!(
        "[exit_hander] task {} enter the exit_handler with code {}",
        tid,
        exit_code
    );
    if task.tid() == INIT_PROCESS_ID {
        if task.pcb().children.is_empty() {
            info!("init_proc exited successfully, exit_code: {}", exit_code);
            return;
        } else {
            error!("init_proc exited before its children!!!");
            let ch_tid: Vec<usize> = task.pcb().children.iter().map(|it| it.tid()).collect();
            error!("child info: {:?}", ch_tid)
        }
    }
    if !task.is_group_leader() {
        // thread resources clean up
        task.thread_group.lock().remove(task.tid());
        TASK_MANAGER.remove(task.tid());
    } else {
        // process resources clean up
        let mut pcb = task.pcb();

        // clear all children
        if !pcb.children.is_empty() {
            for child in pcb.children.iter() {
                // let init_proc take over the child
                let init_proc = TASK_MANAGER.get_init_proc();
                child.pcb().parent = Some(Arc::downgrade(&init_proc));
                init_proc.pcb().children.push(child.clone());
            }
            pcb.children.clear();
        }

        // send SIGCHLD to parent
        if let Some(process) = pcb.parent.clone() {
            let parent = process.upgrade().unwrap();
            let signo: i32 = SigNum::SIGCHLD.into();
            let siginfo = SigInfo {
                signo,
                code: SigCode::User,
                errno: 0,
                extra_info: SigExtraInfo::Extend {
                    si_pid: task.tgid() as u32,
                    si_status: Some(exit_code),
                    si_utime: None,
                    si_stime: None,
                },
            };
            parent.proc_recv_siginfo(siginfo);
        }
        drop(pcb);
    }
    info!(
        "[exit_hander] task {} exited successfully, exit_code: {}, strong_count: {}",
        task.tid(),
        task.exit_code(),
        Arc::strong_count(task)
    );
}
