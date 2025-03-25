use alloc::{sync::Arc, vec::Vec};
use arch::{ArchTrapContext, ArchUserFloatContext};

use crate::{
    signal::{sig_action::{SAFlags, SAHandlerType}, sig_info::SigInfo, sig_num::SigNum, sig_set::SigSet},
    syscall::SyscallResult,
    task::{status::TaskStatus, Task},
};

extern "C" {
    fn user_sigreturn();
}

impl Task {
    pub fn check_signal(self: &Arc<Self>) -> SyscallResult {
        let task = self;
        let mut pending = task.pending_sigs();
        if pending.is_empty() {
            return Ok(0);
        }
        let mut tmp_pending: Vec<SigInfo> = Vec::new();
        let old_mask = self.sig_mask().clone();
        while let Some(sig_info) = pending.pop_with_mask(old_mask) {
            let sa_list = task.sa_list();
            let signo = sig_info.signo;
            trace!("[check_signal] find a signal {}", signo);
            if SigNum::from(signo) != SigNum::SIGKILL
                && SigNum::from(signo) != SigNum::SIGSTOP
                && task.sig_mask().contain_signum(signo as u32)
            {
                info!("[check_signal] sig {} has been blocked", signo);
                tmp_pending.push(sig_info);
                continue;
            }
            let action = sa_list.get(signo as usize).unwrap().clone();
            match action.handler {
                SAHandlerType::Ignore => self.sig_default_ignore(),
                SAHandlerType::Kill => self.sig_default_terminate(),
                SAHandlerType::Stop => self.sig_default_stop(),
                SAHandlerType::Continue => self.sig_default_continue(),
                SAHandlerType::User { handler } => {
                    // save freg
                    task.trap_context_mut().freg_mut().encounter_signal();
                    if !action.flags.contains(SAFlags::SA_NODEFER) {
                        task.sig_mask_mut().enable(signo as u32);
                    };
                    *task.sig_mask_mut() |= action.mask;
                    error!("user handler not implemented, handler addr {}", handler);
                    break;
                }
            }
        }
        Ok(0)
    }
    pub fn set_wake_signal(self: &Arc<Self>, should_wake: SigSet) {
        self.pending_sigs().should_wake = should_wake;
    }
    pub fn recv_siginfo(self: &Arc<Self>, info: SigInfo, thread_only: bool) {
        fn recv_siginfo_inner(task: &Arc<Task>, info: SigInfo) {
            let mut pending = task.pending_sigs();
            let signum = info.signo as u32;
            pending.push(info);
            trace!(
                "[recv_siginfo_inner] tid: {}, push signal {} to pending, status: {:?}",
                task.tid(),
                signum,
                task.status()
            );
            if pending.should_wake.contain_signum(signum) && task.is_suspend() {
                task.wake();
            }
            drop(pending);
        }
        match thread_only {
            true => {
                // is thread
                recv_siginfo_inner(self, info);
            }
            false => {
                // is process (send signal to thread group)
                if !self.is_group_leader() {
                    recv_siginfo_inner(self, info);
                    error!(
                        "send signal to thread group {}, while {} is not group leader",
                        self.tgid(),
                        self.tid()
                    );
                    return;
                }
                let mut guard = self.thread_group.lock();
                let tg = &mut guard.0;
                for it in tg.iter() {
                    let task = it.1.upgrade().unwrap();
                    trace!(
                        "[recv_siginfo] tid: {}, might recv signal {}",
                        task.tid(),
                        info.signo,
                    );
                }
                let mut flag = false;

                for task in tg.iter() {
                    let task = task.1.upgrade().unwrap();
                    if task.sig_mask().contain_signum(info.signo as u32) {
                        continue;
                    }
                    recv_siginfo_inner(&task, info);
                    flag = true;
                    break;
                }
                if !flag {
                    let task = tg.iter().next().unwrap().1.upgrade().unwrap();
                    recv_siginfo_inner(&task, info);
                }
            }
        }
    }
    /// terminate the process
    fn sig_default_terminate(&self) {
        let tg = &self.thread_group.lock().0;
        for (_, t) in tg.iter() {
            let task = t.upgrade().unwrap();
            task.set_status(TaskStatus::Terminated);
        }
    }
    /// stop the process
    fn sig_default_stop(&self) {
        let tg = &self.thread_group.lock().0;
        for (_, t) in tg.iter() {
            let task = t.upgrade().unwrap();
            task.set_status(TaskStatus::Stopped);
        }
    }
    /// continue the process
    fn sig_default_continue(&self) {
        let tg = &self.thread_group.lock().0;
        for (_, t) in tg.iter() {
            let task = t.upgrade().unwrap();
            task.wake();
        }
        // todo: notify parent?
    }
    /// ignore the signal
    #[inline(always)]
    fn sig_default_ignore(&self) {}
}
