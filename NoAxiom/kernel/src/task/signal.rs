use alloc::{sync::Arc, vec::Vec};

use arch::{ArchTrapContext, ArchUserFloatContext};

use crate::{
    signal::{
        sig_action::{SAFlags, SAHandlerType},
        sig_info::SigInfo,
        sig_num::SigNum,
        sig_set::{SigMask, SigSet},
    },
    syscall::SysResult,
    task::{status::TaskStatus, Task},
};

extern "C" {
    fn user_sigreturn();
}

enum SignalControlFlow {
    Continue, // detect default/ignore handler, can handle other signals
    Break,    // detect user handler, should do restore
}

impl Task {
    fn handle_signal(self: &Arc<Self>, si: SigInfo) -> SignalControlFlow {
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
                    self.sig_mask_mut().enable(si.signo as u32);
                };
                *self.sig_mask_mut() |= action.mask;
                self.trap_context_mut().freg_mut().encounter_signal(); // save freg
                return SignalControlFlow::Break;
            }
        }
        SignalControlFlow::Continue
    }
    pub fn check_signal(self: &Arc<Self>) -> SysResult<Option<SigMask>> {
        let mut pending = self.pending_sigs();
        if pending.is_empty() {
            return Ok(None);
        }
        let mut blocked_pending: Vec<SigInfo> = Vec::new();
        let old_mask = self.sig_mask().clone();
        let res = loop {
            match pending.pop_with_mask(old_mask) {
                Some(si) => {
                    trace!("[check_signal] find a signal {}", si.signo);
                    let signum = SigNum::from(si.signo);
                    // kill / stop signal cannot be blocked
                    // other signals can be blocked by sigmask
                    // todo: maybe can disable sigmask's kill/stop bits to avoid this check?
                    if signum != SigNum::SIGKILL
                        && signum != SigNum::SIGSTOP
                        && self.sig_mask().contain_signum(si.signo as u32)
                    {
                        // current signum is blocked by sigmask
                        info!("[check_signal] sig {}: blocked", si.signo);
                        blocked_pending.push(si);
                    } else {
                        // successfully recived signal, handle it
                        info!("[check_signal] sig {}: start to handle", si.signo);
                        match self.handle_signal(si) {
                            // if detect user sigaction, break with old_mask(should be restored)
                            SignalControlFlow::Break => break Some(old_mask),
                            SignalControlFlow::Continue => continue,
                        }
                    }
                }
                None => break None,
            }
        };
        for it in blocked_pending.into_iter() {
            pending.push(it);
        }
        Ok(res)
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
            if pending.should_wake.contain_signum(signum) {
                task.wake_checked();
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
            task.wake_unchecked();
        }
        // todo: notify parent?
    }
    /// ignore the signal
    #[inline(always)]
    fn sig_default_ignore(&self) {}
}
