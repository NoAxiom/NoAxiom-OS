use alloc::{sync::Arc, vec::Vec};

use arch::{ArchTrapContext, ArchUserFloatContext};
use ksync::mutex::{SpinLock, SpinLockGuard};

use super::task::TaskInner;
use crate::{
    signal::{
        sig_action::{SAFlags, SAHandlerType},
        sig_info::SigInfo,
        sig_num::SigNum,
        sig_set::SigMask,
    },
    syscall::SysResult,
    task::{status::TaskStatus, Task},
};

extern "C" {
    fn user_sigreturn();
}

impl Task {
    pub fn check_signal(self: &Arc<Self>) -> SysResult<Option<SigMask>> {
        let mut pcb = self.pcb();
        if pcb.pending_sigs.is_empty() {
            return Ok(None);
        }
        let mut blocked_pending: Vec<SigInfo> = Vec::new();
        let old_mask = pcb.pending_sigs.sig_mask.clone();
        let res = loop {
            match pcb.pending_sigs.pop_with_mask(old_mask) {
                Some(si) => {
                    trace!("[check_signal] find a signal {}", si.signo);
                    let signum = SigNum::from(si.signo);
                    // kill / stop signal cannot be blocked
                    // other signals can be blocked by sigmask
                    // todo: maybe can disable sigmask's kill/stop bits to avoid this check?
                    if signum != SigNum::SIGKILL
                        && signum != SigNum::SIGSTOP
                        && old_mask.contain_signum(si.signo as u32)
                    {
                        // current signum is blocked by sigmask
                        info!("[check_signal] sig {}: blocked", si.signo);
                        blocked_pending.push(si);
                    } else {
                        // successfully recived signal, handle it
                        info!("[check_signal] sig {}: start to handle", si.signo);
                        let sa_list = self.sa_list();
                        let signum = SigNum::from(si.signo);
                        let action = sa_list.get(signum).unwrap().clone();
                        match action.handler {
                            SAHandlerType::Ignore => self.sig_default_ignore(),
                            SAHandlerType::Kill => self.sig_default_terminate(),
                            SAHandlerType::Stop => self.sig_default_stop(),
                            SAHandlerType::Continue => self.sig_default_continue(),
                            SAHandlerType::User { handler } => {
                                info!("[handle_signal] start to handle user sigaction");
                                if !action.flags.contains(SAFlags::SA_NODEFER) {
                                    pcb.pending_sigs.sig_mask.enable(si.signo as u32);
                                };
                                pcb.pending_sigs.sig_mask |= action.mask;
                                self.trap_context_mut().freg_mut().encounter_signal(); // save freg
                                break Some(old_mask);
                            }
                        }
                    }
                }
                None => break None,
            }
        };
        Ok(res)
    }

    /// siginfo receiver with thread checked
    pub fn recv_siginfo(
        self: &Arc<Self>,
        pcb: &mut SpinLockGuard<TaskInner>,
        si: SigInfo,
        thread_only: bool,
    ) {
        match thread_only {
            true => {
                // is thread
                self.recv_siginfo_inner(pcb, si);
            }
            false => {
                // is process (send signal to thread group)
                if !self.is_group_leader() {
                    error!(
                        "send signal to thread group {}, while {} is not group leader",
                        self.tgid(),
                        self.tid()
                    );
                }
                let mut guard = self.thread_group.lock();
                let tg = &mut guard.0;
                for it in tg.iter() {
                    let task = it.1.upgrade().unwrap();
                    trace!(
                        "[recv_siginfo] tid: {}, might recv signal {}",
                        task.tid(),
                        si.signo,
                    );
                }
                let mut flag = false;

                for task in tg.iter() {
                    let task = task.1.upgrade().unwrap();
                    if pcb.pending_sigs.sig_mask.contain_signum(si.signo as u32) {
                        continue;
                    }
                    task.recv_siginfo_inner(pcb, si);
                    flag = true;
                    break;
                }
                if !flag {
                    let task = tg.iter().next().unwrap().1.upgrade().unwrap();
                    task.recv_siginfo_inner(pcb, si)
                }
            }
        }
    }

    /// a raw siginfo receiver without thread checked
    fn recv_siginfo_inner(self: &Arc<Task>, pcb: &mut SpinLockGuard<TaskInner>, info: SigInfo) {
        let signum = info.signo as u32;
        pcb.pending_sigs.push(info);
        trace!(
            "[recv_siginfo_inner] tid: {}, push signal {} to pending",
            self.tid(),
            signum,
        );
        if pcb.pending_sigs.should_wake.contain_signum(signum) && pcb.can_wake() {
            self.wake_unchecked();
        }
    }

    /// terminate the process
    fn sig_default_terminate(&self) {
        let tg = &self.thread_group.lock().0;
        for (_, t) in tg.iter() {
            let task = t.upgrade().unwrap();
            task.pcb().set_status(TaskStatus::Terminated);
        }
    }
    /// stop the process
    fn sig_default_stop(&self) {
        let tg = &self.thread_group.lock().0;
        for (_, t) in tg.iter() {
            let task = t.upgrade().unwrap();
            task.pcb().set_status(TaskStatus::Stopped);
        }
    }
    /// continue the process
    fn sig_default_continue(&self) {
        let tg = &self.thread_group.lock().0;
        for (_, t) in tg.iter() {
            let task = t.upgrade().unwrap();
            task.wake_unchecked();
        }
        // todo: notify parent?
    }
    /// ignore the signal
    #[inline(always)]
    fn sig_default_ignore(&self) {}
}
