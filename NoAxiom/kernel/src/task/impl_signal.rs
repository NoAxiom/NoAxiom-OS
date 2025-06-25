use alloc::{sync::Arc, vec::Vec};
use core::mem::size_of;

use arch::{ArchTrapContext, ArchUserFloatContext, TrapArgs};
use config::mm::SIG_TRAMPOLINE;
use ksync::assert_no_lock;

use crate::{
    mm::user_ptr::UserPtr,
    signal::{
        sig_action::{SAFlags, SAHandlerType},
        sig_info::SigInfo,
        sig_num::SigNum,
        sig_set::SigMask,
        sig_stack::{MContext, UContext},
    },
    task::{exit::ExitCode, status::TaskStatus, Task},
};

extern "C" {
    /// user_sigreturn: we set RA to this function before entering sig hander
    /// when returning from user sigaction handler,
    /// it will jump to this func and trigger syscall_SIGRETURN
    pub fn user_sigreturn();
}

impl Task {
    pub fn peek_has_pending_signal(self: &Arc<Self>, mask: &SigMask) -> bool {
        let pcb = self.pcb();
        let mask = pcb.sig_mask() | *mask;
        pcb.pending_sigs.has_expect_signals(!mask)
    }
    pub async fn check_signal(self: &Arc<Self>, tmp_mask: Option<SigMask>) {
        let mut pcb = self.pcb();
        let old_mask = tmp_mask.unwrap_or(pcb.pending_sigs.sig_mask.clone());
        trace!(
            "[check_signal] tid: {}, check pending signals, old_mask: {:?}",
            self.tid(),
            old_mask
        );
        if !pcb.pending_sigs.has_expect_signals(!old_mask) {
            return;
        }
        let mut pending = Vec::new();
        while let Some(si) = pcb.pending_sigs.pop_with_mask(old_mask) {
            trace!("[check_signal] find a signal {}", si.signo);
            pending.push(si);
        }
        drop(pcb);

        let mut actions = Vec::with_capacity(pending.len());
        let sa_list = self.sa_list();
        for si in pending.iter() {
            let signum = SigNum::from(si.signo);
            let action = sa_list.get(signum).unwrap().clone();
            actions.push(action);
        }
        drop(sa_list);

        debug!(
            "[check_signal] pending signals: {}",
            pending
                .iter()
                .enumerate()
                .map(|(i, si)| format!("{:?}: {:?}", actions[i].handler, SigNum::from(si.signo)))
                .collect::<Vec<_>>()
                .join(", ")
        );

        for (i, si) in pending.into_iter().enumerate() {
            let signum = SigNum::from(si.signo);
            let action = actions[i];
            info!(
                "[check_signal] sig {:?}: start to handle, handler: {:?}",
                signum, action.handler
            );
            match action.handler {
                SAHandlerType::Ignore => self.sig_default_ignore(),
                SAHandlerType::Kill => self.sig_default_terminate(),
                SAHandlerType::Stop => self.sig_default_stop(),
                SAHandlerType::Continue => self.sig_default_continue(),
                SAHandlerType::User { handler } => {
                    let mut pcb = self.pcb();
                    info!(
                        "[handle_signal] start to handle user sigaction, signum: {}, handler: {:#x}, flags: {:?}",
                        si.signo, handler, action.flags
                    );
                    if !action.flags.contains(SAFlags::SA_NODEFER) {
                        pcb.pending_sigs.sig_mask.enable(si.signo as u32);
                    };
                    pcb.pending_sigs.sig_mask |= action.mask;

                    use TrapArgs::*;
                    let cx = self.trap_context_mut();
                    cx.freg_mut().encounter_signal(); // save freg

                    // if detect user-defined stack, use it at first
                    let sp = match pcb.sig_stack.take() {
                        Some(s) => {
                            error!("[sigstack] user defined signal stack. unimplemented!");
                            s.stack_top()
                        }
                        None => cx[SP],
                    };
                    let uc_stack = pcb.sig_stack.unwrap_or_default();
                    drop(pcb);

                    // write ucontext
                    let mut new_sp = sp - size_of::<UContext>();
                    let ucontext_ptr: UserPtr<UContext> = new_sp.into();
                    let ucontext = UContext {
                        uc_flags: 0,
                        uc_link: 0,
                        // fixme: always returns default here
                        uc_stack,
                        uc_sigmask: old_mask,
                        __unused: [0; 1024 / 8 - core::mem::size_of::<SigMask>()],
                        uc_mcontext: MContext::from_cx(&cx),
                    };
                    assert_no_lock!();
                    ucontext_ptr.write(ucontext).await.unwrap_or_else(|err| {
                        error!("[sigstack] write ucontext failed: {:?}", err);
                    });
                    *self.ucx_mut() = new_sp.into();
                    cx[A0] = si.signo as usize;

                    // write sig_info
                    if action.flags.contains(SAFlags::SA_SIGINFO) {
                        cx[A2] = new_sp;
                        #[derive(Default, Copy, Clone)]
                        #[repr(C)]
                        pub struct LinuxSigInfo {
                            pub si_signo: i32,
                            pub si_errno: i32,
                            pub si_code: i32,
                            pub _pad: [i32; 29],
                            _align: [u64; 0],
                        }
                        let mut siginfo_v = LinuxSigInfo::default();
                        siginfo_v.si_signo = si.signo;
                        siginfo_v.si_code = si.code as i32;
                        new_sp -= size_of::<LinuxSigInfo>();
                        let siginfo_ptr: UserPtr<LinuxSigInfo> = new_sp.into();
                        assert_no_lock!();
                        siginfo_ptr.try_write(siginfo_v).await.unwrap();
                        cx[A1] = new_sp;
                    }

                    // update cx
                    // flow: kernel (restore) -> handler -> ..
                    // -> user_sigreturn -> (syscall) kernel
                    // fixme: should we update gp & tp?
                    cx[EPC] = handler;
                    cx[RA] = if action.flags.contains(SAFlags::SA_RESTORER) {
                        info!("[sigstack] use restorer: {:#x}", action.restorer);
                        action.restorer
                    } else {
                        info!("[sigstack] use default return: {:#x}", SIG_TRAMPOLINE);
                        SIG_TRAMPOLINE
                    };
                    cx[SP] = new_sp;

                    // fixme: this return could cause signal missing
                    return;
                }
            }
        }
    }

    /// siginfo receiver with thread checked
    pub fn recv_siginfo(self: &Arc<Self>, si: SigInfo, thread_only: bool) {
        match thread_only {
            true => {
                // is thread
                self.try_recv_siginfo_inner(si, true);
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
                let mut guard = self.thread_group();
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
                    if task.try_recv_siginfo_inner(si, false) {
                        flag = true;
                        break;
                    }
                }
                if !flag {
                    let task = tg.iter().next().unwrap().1.upgrade().unwrap();
                    task.try_recv_siginfo_inner(si, true);
                }
            }
        }
    }

    /// a raw siginfo receiver without thread checked
    fn try_recv_siginfo_inner(self: &Arc<Task>, info: SigInfo, forced: bool) -> bool {
        let mut pcb = self.pcb();
        if pcb.pending_sigs.sig_mask.contain_signum(info.signo as u32) && !forced {
            return false;
        }
        let signum = info.signo as u32;
        pcb.pending_sigs.push(info);
        warn!(
            "[recv_siginfo_inner] tid: {}, push signal {} to pending during syscall {:?}",
            self.tid(),
            signum,
            self.tcb().current_syscall,
        );
        if pcb.pending_sigs.should_wake.contain_signum(signum) {
            warn!("[recv_siginfo_inner] tid: {}, wake up task", self.tid());
            self.wake_unchecked();
        } else {
            warn!(
                "[recv_siginfo_inner] wake task {} get blocked, signo: {:?}, mask: {:?}",
                self.tid(),
                info.signo,
                pcb.sig_mask()
            )
        }
        return true;
    }

    /// terminate the process
    fn sig_default_terminate(&self) {
        warn!(
            "sig_default_terminate: terminate the process, tid: {}, during: {:?}",
            self.tid(),
            self.tcb().current_syscall
        );
        debug!("[sig_default_terminate] terminate the process");
        self.terminate_group(ExitCode::default());
        debug!("[sig_default_terminate] terminate the process done");
    }
    /// stop the process
    fn sig_default_stop(&self) {
        warn!(
            "sig_default_stop: stop the process, tid: {}, during: {:?}",
            self.tid(),
            self.tcb().current_syscall
        );
        let tg = self.thread_group();
        for (_, t) in tg.0.iter() {
            let task = t.upgrade().unwrap();
            task.pcb().set_status(TaskStatus::Stopped);
        }
    }
    /// continue the process
    fn sig_default_continue(&self) {
        warn!(
            "sig_default_continue: continue the process, tid: {}, during: {:?}",
            self.tid(),
            self.tcb().current_syscall
        );
        let tg = self.thread_group();
        for (_, t) in tg.0.iter() {
            let task = t.upgrade().unwrap();
            task.wake_unchecked();
        }
        // todo: notify parent?
    }
    /// ignore the signal
    #[inline(always)]
    fn sig_default_ignore(&self) {}
}
