//! sys_sigaction
//! sys_sigprocmask
//! sys_kill
//! sys_sigreturn
//! sys_sigsuspend

use super::{Syscall, SyscallResult};
use crate::{
    constant::signal::MAX_SIGNUM,
    include::result::Errno,
    mm::user_ptr::UserPtr,
    signal::{
        sig_action::{KSigAction, SigAction},
        sig_num::{SigNum, Signo},
    },
};

impl Syscall<'_> {
    pub fn sys_sigaction(&self, signo: Signo, act: usize, old_act: usize) -> SyscallResult {
        debug!(
            "[sys_sigaction]: signum {}, new act ptr {:#x}, old act ptr {:#x}",
            signo, act, old_act,
        );

        let act = UserPtr::<SigAction>::new(act);
        let old_act = UserPtr::<SigAction>::new(old_act);
        let task = self.task;
        let signum = SigNum::from(signo);

        // signum out of range
        if signo >= MAX_SIGNUM as i32 || signum == SigNum::SIGKILL || signum == SigNum::SIGSTOP {
            return Err(Errno::EINVAL);
        }

        // when detect old sig action request, write the swapped sigaction into old_act
        if !old_act.is_null() {
            let sa = task.signal().sa_list.lock();
            let old = sa.get(signum as usize).unwrap();
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

    pub fn sys_sigreturn() {
        unimplemented!()
    }
}
