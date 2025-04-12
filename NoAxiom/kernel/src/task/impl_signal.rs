use alloc::sync::Arc;
use core::mem::size_of;

use arch::{ArchTrapContext, ArchUserFloatContext, TrapArgs};
use ksync::mutex::SpinLockGuard;

use super::task::PCB;
use crate::{
    mm::user_ptr::UserPtr,
    signal::{
        sig_action::{SAFlags, SAHandlerType},
        sig_info::SigInfo,
        sig_num::SigNum,
        sig_set::SigMask,
        sig_stack::{MContext, UContext},
    },
    task::{status::TaskStatus, Task},
};

extern "C" {
    /// user_sigreturn: we set RA to this function before entering sig hander
    /// when returning from user sigaction handler,
    /// it will jump to this func and trigger syscall_SIGRETURN
    fn user_sigreturn();
}

impl Task {
    pub fn check_signal(self: &Arc<Self>) -> Option<SigMask> {
        let mut pcb = self.pcb();
        if pcb.pending_sigs.is_empty() {
            return None;
        }
        let old_mask = pcb.pending_sigs.sig_mask.clone();
        while let Some(si) = pcb.pending_sigs.pop_with_mask(old_mask) {
            trace!("[check_signal] find a signal {}", si.signo);
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

                    // write ucontext
                    let mut new_sp = sp - size_of::<UContext>();
                    let ucontext_ptr: UserPtr<UContext> = new_sp.into();
                    let ucontext = UContext {
                        uc_flags: 0,
                        uc_link: 0,
                        // fixme: always returns default here
                        uc_stack: pcb.sig_stack.unwrap_or_default(),
                        uc_sigmask: old_mask,
                        uc_sig: [0; 16],
                        uc_mcontext: MContext::from_cx(&cx),
                    };
                    ucontext_ptr.write(ucontext);
                    pcb.ucontext_ptr = new_sp.into();
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
                        siginfo_ptr.write(siginfo_v);
                        cx[A1] = new_sp;
                    }

                    // update cx
                    // flow: kernel (restore) -> handler -> ..
                    // -> user_sigreturn -> (syscall) kernel
                    // fixme: should we update gp & tp?
                    cx[EPC] = handler;
                    cx[RA] = user_sigreturn as usize;
                    cx[SP] = new_sp;

                    return Some(old_mask);
                }
            }
        }
        None
    }

    /// siginfo receiver with thread checked
    pub fn recv_siginfo(
        self: &Arc<Self>,
        pcb: &mut SpinLockGuard<PCB>,
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
    fn recv_siginfo_inner(self: &Arc<Task>, pcb: &mut SpinLockGuard<PCB>, info: SigInfo) {
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
        let tg = self.thread_group();
        for (_, t) in tg.0.iter() {
            let task = t.upgrade().unwrap();
            task.pcb().set_status(TaskStatus::Terminated);
        }
    }
    /// stop the process
    fn sig_default_stop(&self) {
        let tg = self.thread_group();
        for (_, t) in tg.0.iter() {
            let task = t.upgrade().unwrap();
            task.pcb().set_status(TaskStatus::Stopped);
        }
    }
    /// continue the process
    fn sig_default_continue(&self) {
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
