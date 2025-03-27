//! sys_sigaction
//! sys_sigprocmask
//! sys_kill
//! sys_sigreturn
//! sys_sigsuspend

use arch::{ArchTrapContext, TrapArgs};

use super::{Syscall, SyscallResult};
use crate::{
    constant::signal::MAX_SIGNUM,
    include::result::Errno,
    mm::user_ptr::UserPtr,
    signal::{
        sig_action::{KSigAction, USigAction},
        sig_num::{SigNum, Signo},
        sig_set::SigSet,
        sig_stack::UContext,
    },
};

impl Syscall<'_> {
    pub fn sys_sigaction(&self, signo: Signo, act: usize, old_act: usize) -> SyscallResult {
        debug!(
            "[sys_sigaction]: signum {}, new act ptr {:#x}, old act ptr {:#x}",
            signo, act, old_act,
        );

        let act = UserPtr::<USigAction>::new(act);
        let old_act = UserPtr::<USigAction>::new(old_act);
        let task = self.task;
        let signum = SigNum::from(signo);

        // signum out of range
        if signo >= MAX_SIGNUM as i32 || signum == SigNum::SIGKILL || signum == SigNum::SIGSTOP {
            return Err(Errno::EINVAL);
        }

        // when detect old sig action request, write the swapped sigaction into old_act
        if !old_act.is_null() {
            let sa = task.sa_list();
            let old = sa.get(signum).unwrap();
            old_act.write(old.into_sa());
        }

        // when detect new sig action, register it into pcb
        if !act.is_null() {
            let sa = act.read();
            task.sa_list()
                .set_sigaction(signum as usize, KSigAction::from_sa(sa, signum));
        }
        Ok(0)
    }

    pub fn sys_sigreturn(&self) -> SyscallResult {
        use TrapArgs::*;

        let task = self.task;
        let cx = task.trap_context_mut();
        let mut pcb = task.pcb();

        let ucontext_ptr: UserPtr<UContext> = pcb.ucontext_ptr;
        let ucontext = ucontext_ptr.read();
        *pcb.sig_mask_mut() = ucontext.uc_sigmask;
        pcb.sig_stack = (ucontext.uc_stack.ss_size != 0).then_some(ucontext.uc_stack);
        cx[EPC] = ucontext.uc_mcontext.epc();
        *cx.gprs_mut() = ucontext.uc_mcontext.gprs();

        // fixme: why return A0?
        Ok(cx[A0] as isize)
    }

    pub fn sys_sigprocmask(
        &self,
        how: usize,
        set: usize,
        old_set: usize,
        sigset_size: usize,
    ) -> SyscallResult {
        const SIGBLOCK: usize = 0;
        const SIGUNBLOCK: usize = 1;
        const SIGSETMASK: usize = 2;
        if sigset_size != 8 {
            error!("[sys_sigprocmask] sigset_size isn't 8");
        }

        let task = self.task;
        let set = UserPtr::<SigSet>::new(set);
        let old_set = UserPtr::<SigSet>::new(old_set);
        let mut pcb = task.pcb();

        if !old_set.is_null() {
            old_set.write(pcb.sig_mask());
        }
        if !set.is_null() {
            let mut set = set.read();
            log::info!("[sys_rt_sigprocmask] set:{set:#x}");
            // It is not possible to block SIGKILL or SIGSTOP.  Attempts to do so are
            // silently ignored.
            set.remove(SigSet::SIGKILL | SigSet::SIGCONT);
            match how {
                SIGBLOCK => {
                    *pcb.sig_mask_mut() |= set;
                }
                SIGUNBLOCK => {
                    pcb.sig_mask_mut().remove(set);
                }
                SIGSETMASK => {
                    *pcb.sig_mask_mut() = set;
                }
                _ => {
                    return Err(Errno::EINVAL);
                }
            };
        }
        Ok(0)
    }
}
