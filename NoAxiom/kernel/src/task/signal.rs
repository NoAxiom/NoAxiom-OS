use alloc::{sync::Arc, vec::Vec};
use core::mem::size_of;

use arch::{ArchTrapContext, ArchUserFloatContext, TrapArgs};
use config::mm::SIG_TRAMPOLINE;
use ksync::assert_no_lock;

use crate::{
    include::process::TaskFlags,
    mm::user_ptr::UserPtr,
    signal::{
        sig_action::{SAFlags, SAHandlerType},
        sig_detail::SigDetail,
        sig_info::SigInfo,
        sig_set::SigMask,
        sig_stack::{MContext, UContext},
        signal::Signal,
    },
    syscall::utils::clear_current_syscall,
    task::{exit::ExitReason, status::TaskStatus, Task},
};

extern "C" {
    /// user_sigreturn: we set RA to this function before entering sig hander
    /// when returning from user sigaction handler,
    /// it will jump to this func and trigger syscall_SIGRETURN
    pub fn user_sigreturn();
}

impl Task {
    pub async fn check_signal(self: &Arc<Self>) {
        // check tif first
        if !self.tif().contains(TaskFlags::TIF_NOTIFY_SIGNAL) {
            return;
        }

        // check pendingn signal
        let mut pcb = self.pcb();
        let sig_mask = self.sig_mask();
        if !pcb.signals.has_pending_signals(sig_mask) {
            return;
        }
        let mut pending = Vec::new();
        while let Some(si) = pcb.signals.pop_with_mask(sig_mask) {
            pending.push(si);
        }
        drop(pcb);

        // restore sigmask
        if self.tif().contains(TaskFlags::TIF_RESTORE_SIGMASK) {
            if let Some(mask) = self.take_old_mask() {
                info!(
                    "[check_signal] restore old sigmask: {}",
                    mask.debug_info_short()
                );
                self.set_sig_mask(mask);
            } else {
                warn!("[check_signal] no old sigmask to restore while TIF_RESTORE_SIGMASK is set");
            }
        }

        // handle each signal
        let sa_list = self.sa_list();
        for si in pending {
            let signal = si.signal;
            let action = sa_list[signal].clone();
            info!(
                "[check_signal] sig {:?}: start to handle, handler: {:?}",
                signal, action.handler
            );

            // check interrpt syscall
            let tcb = self.tcb_mut();
            if action.flags.contains(SAFlags::SA_RESTART)
                && tcb.flags.contains(TaskFlags::TIF_SIGPENDING)
            {
                warn!(
                    "TID{} restart syscall {:?} after signal: {:?}",
                    self.tid(),
                    self.tcb().current_syscall,
                    signal
                );
                self.trap_context_mut()[TrapArgs::EPC] -= 4;
                self.clear_syscall_result();
                clear_current_syscall();
                tcb.flags -= TaskFlags::TIF_SIGPENDING;
                // todo: current syscall might be incorrect after sigreturn
            }

            // start handle
            match action.handler {
                SAHandlerType::Ignore => self.sig_default_ignore(),
                SAHandlerType::Kill => self.sig_default_terminate(si.errno, si.signal),
                SAHandlerType::Stop => self.sig_default_stop(),
                SAHandlerType::Continue => self.sig_default_continue(),
                SAHandlerType::User { handler } => {
                    drop(sa_list);
                    let mut pcb = self.pcb();
                    info!(
                        "[handle_signal] start to handle user sigaction, signal: {:?}, handler: {:#x}, flags: {:?}",
                        si.signal, handler, action.flags
                    );
                    if !action.flags.contains(SAFlags::SA_NODEFER) {
                        self.sig_mask_mut().enable(si.signal);
                    };
                    *self.sig_mask_mut() |= action.mask;

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
                        uc_sigmask: sig_mask,
                        __unused: [0; 1024 / 8 - core::mem::size_of::<SigMask>()],
                        uc_mcontext: MContext::from_cx(&cx),
                    };
                    assert_no_lock!();
                    ucontext_ptr.write(ucontext).await.unwrap_or_else(|err| {
                        error!("[sigstack] write ucontext failed: {:?}", err);
                    });
                    *self.ucx_mut() = new_sp.into();
                    cx[A0] = si.signal.raw();

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
                        siginfo_v.si_signo = si.signal.raw() as i32;
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
                    return;
                }
            }
        }
    }

    /// siginfo receiver with thread checked
    pub fn recv_siginfo(self: &Arc<Self>, si: SigInfo, thread_only: bool) -> bool {
        match thread_only {
            true => {
                // is thread
                self.try_recv_siginfo_inner(si, true)
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
                if tg.is_empty() {
                    error!("[recv_siginfo] thread group is empty, tid: {}", self.tid());
                    return false;
                }
                for it in tg.iter() {
                    let task = it.1.upgrade().unwrap();
                    trace!(
                        "[recv_siginfo] tid: {}, might recv signal {:?}",
                        task.tid(),
                        si.signal,
                    );
                }
                for task in tg.iter() {
                    if let Some(task) = task.1.upgrade() {
                        if task.try_recv_siginfo_inner(si, false) {
                            return true;
                        }
                    }
                }
                if let Some((_, task)) = tg.iter().next() {
                    if let Some(task) = task.upgrade() {
                        return task.try_recv_siginfo_inner(si, true);
                    }
                }
                false
            }
        }
    }

    /// a raw siginfo receiver without thread checked
    fn try_recv_siginfo_inner(self: &Arc<Task>, info: SigInfo, forced: bool) -> bool {
        let mut pcb = self.pcb();
        if self.sig_mask().contains_signal(info.signal) && !forced {
            return false;
        }
        let signal = info.signal;
        pcb.signals.push(info);
        warn!(
            "[recv_siginfo_inner] tid: {}, push signal {:?} to pending during syscall {:?}",
            self.tid(),
            signal,
            self.tcb().current_syscall,
        );
        if pcb.signals.should_wake.contains_signal(signal) {
            warn!("[recv_siginfo_inner] tid: {}, wake up task", self.tid());
            self.wake_unchecked();
        } else {
            warn!(
                "[recv_siginfo_inner] wake for task {} get blocked, signal: {:?}, mask: {}, wakeset: {}",
                self.tid(),
                info.signal,
                self.sig_mask().debug_info_short(),
                pcb.signals.should_wake.debug_info_short(),
            );
            if let SigDetail::Kill(x) = info.detail {
                warn!("[recv_siginfo_inner] kill sender: {}", x.pid);
            };
        }
        // notify the task to check signal
        self.tif_mut().insert(TaskFlags::TIF_NOTIFY_SIGNAL);
        return true;
    }

    /// terminate the process
    fn sig_default_terminate(&self, errno: i32, signal: Signal) {
        warn!(
            "sig_default_terminate: terminate the process, tid: {}, during: {:?}",
            self.tid(),
            self.tcb().current_syscall
        );
        debug!("[sig_default_terminate] terminate the process");
        self.terminate_group(ExitReason::new(errno, signal.raw()));
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
            task.pcb().set_status(TaskStatus::Stopped, task.tif_mut());
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
