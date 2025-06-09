//! sys_sigaction
//! sys_sigprocmask
//! sys_kill
//! sys_sigreturn
//! sys_sigsuspend

use arch::{ArchTrapContext, TrapArgs};

use super::{Syscall, SyscallResult};
use crate::{
    config::task::INIT_PROCESS_ID,
    constant::signal::MAX_SIGNUM,
    include::result::Errno,
    mm::user_ptr::UserPtr,
    sched::utils::suspend_now,
    signal::{
        sig_action::{KSigAction, USigAction},
        sig_detail::{SigDetail, SigKillDetail},
        sig_info::{SigCode, SigInfo},
        sig_num::{SigNum, Signo},
        sig_set::SigSet,
    },
    task::manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
};

impl Syscall<'_> {
    pub async fn sys_sigaction(&self, signo: Signo, act: usize, old_act: usize) -> SyscallResult {
        debug!(
            "[sys_sigaction]: signum {:?}, new act ptr {:#x}, old act ptr {:#x}",
            SigNum::from(signo),
            act,
            old_act,
        );

        let signum = SigNum::from(signo);
        if signo >= MAX_SIGNUM as i32 || signum == SigNum::SIGKILL || signum == SigNum::SIGSTOP {
            // signum out of range
            return Err(Errno::EINVAL);
        }

        let act = UserPtr::<USigAction>::new(act);
        let old_act = UserPtr::<USigAction>::new(old_act);
        let task = self.task;
        let act = act.try_read().await?;

        let mut sa = task.sa_list();
        let old = sa.get(signum).unwrap().into_sa();
        // when detect new sig action, register it into sigaction list
        if let Some(act) = act {
            sa.set_sigaction(signum as usize, KSigAction::from_sa(act, signum));
        }
        drop(sa);

        // when detect old sig action request, write the swapped sigaction into old_act
        old_act.try_write(old).await?;

        Ok(0)
    }

    pub async fn sys_sigreturn(&self) -> SyscallResult {
        use TrapArgs::*;
        info!("[sigreturn] do signal return");

        let task = self.task;
        let ucontext_ptr = task.ucx();
        let ucontext = ucontext_ptr.read().await?;
        let cx = task.trap_context_mut();

        let mut pcb = task.pcb();
        *pcb.sig_mask_mut() = ucontext.uc_sigmask;
        pcb.sig_stack = (ucontext.uc_stack.ss_size != 0).then_some(ucontext.uc_stack);
        cx[EPC] = ucontext.uc_mcontext.epc();
        *cx.gprs_mut() = ucontext.uc_mcontext.gprs();
        info!("[sys_sigreturn] cx: {:#x?}", cx);
        drop(pcb);

        Ok(cx[RES] as isize)
    }

    pub async fn sys_sigprocmask(
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
        let set_value = set.try_read().await?;
        let old_set = UserPtr::<SigSet>::new(old_set);
        debug!(
            "[sys_sigprocmask] tid: {}, how: {}, set: {:?}",
            task.tid(),
            how,
            set_value,
        );

        let mut pcb = task.pcb();
        let old_sigmask = pcb.sig_mask();
        if let Some(mut set) = set_value {
            // sigmask shouldn't contain SIGKILL and SIGCONT
            set.remove(SigSet::SIGKILL | SigSet::SIGCONT);
            match how {
                SIGBLOCK => *pcb.sig_mask_mut() |= set,
                SIGUNBLOCK => *pcb.sig_mask_mut() &= !set,
                SIGSETMASK => *pcb.sig_mask_mut() = set,
                _ => return Err(Errno::EINVAL),
            };
        }
        drop(pcb);
        old_set.try_write(old_sigmask).await?;
        Ok(0)
    }

    pub fn sys_kill(&self, pid: isize, signo: i32) -> SyscallResult {
        if signo == 0 {
            return Ok(0);
        }
        let sig = SigNum::from(signo);
        if sig == SigNum::INVALID {
            return Err(Errno::EINVAL);
        }
        warn!(
            "[sys_kill] from: {}, target: {}, signo: {}, sig_name: {:?}",
            self.task.tid(),
            pid,
            signo,
            sig
        );
        match pid {
            0 => {
                // process group
                let pgid = self.task.get_pgid();
                for task in PROCESS_GROUP_MANAGER
                    .lock()
                    .get_group(pgid)
                    .unwrap()
                    .into_iter()
                    .map(|t| t.task())
                {
                    task.recv_siginfo(
                        SigInfo::new_detailed(
                            signo,
                            SigCode::User,
                            0,
                            SigDetail::Kill(SigKillDetail { pid: pgid }),
                        ),
                        false,
                    );
                }
            }
            -1 => {
                for (_, task) in TASK_MANAGER.0.lock().iter() {
                    let task = task.upgrade().unwrap();
                    if task.tgid() != INIT_PROCESS_ID && task.is_group_leader() {
                        task.recv_siginfo(
                            SigInfo::new_detailed(
                                signo,
                                SigCode::User,
                                0,
                                SigDetail::Kill(SigKillDetail { pid: task.tgid() }),
                            ),
                            false,
                        );
                    }
                }
            }
            _ if pid > 0 => {
                if let Some(task) = TASK_MANAGER.get(pid as usize) {
                    if task.is_group_leader() {
                        task.recv_siginfo(
                            SigInfo::new_detailed(
                                signo,
                                SigCode::User,
                                0,
                                SigDetail::Kill(SigKillDetail { pid: task.tgid() }),
                            ),
                            false,
                        );
                    } else {
                        // sys_kill is sent to process not thread
                        return Err(Errno::ESRCH);
                    }
                } else {
                    return Err(Errno::ESRCH);
                }
            }
            _ => {
                // pid < -1
                // sig is sent to every process in the process group whose ID is -pid.
                let pgid = self.task.get_pgid();
                for task in PROCESS_GROUP_MANAGER
                    .lock()
                    .get_group(pgid)
                    .unwrap()
                    .into_iter()
                    .map(|t| t.task())
                {
                    if task.tgid() == -pid as usize {
                        task.recv_siginfo(
                            SigInfo::new_detailed(
                                signo,
                                SigCode::User,
                                0,
                                SigDetail::Kill(SigKillDetail { pid: pgid }),
                            ),
                            false,
                        );
                        return Ok(0);
                    }
                }
                return Err(Errno::ESRCH);
            }
        }
        Ok(0)
    }

    pub async fn sys_sigsuspend(&self, mask: usize) -> SyscallResult {
        let mask = UserPtr::<SigSet>::from(mask);
        let task = self.task;
        let mut mask = mask.read().await?;
        mask.remove(SigSet::SIGKILL | SigSet::SIGSTOP);
        let invoke_signal = task.sa_list().get_user_bitmap();
        debug!(
            "[sys_sigsuspend] tid: {}, new_mask: {:?}, invoke_signal: {:?}",
            task.tid(),
            mask,
            invoke_signal
        );
        let mut pcb = task.pcb();
        let old_mask = core::mem::replace(&mut pcb.sig_mask(), mask);
        let expect = mask | invoke_signal;
        if pcb.pending_sigs.has_expect_signals(expect) {
            return Err(Errno::EINTR);
        } else {
            *pcb.sig_mask_mut() = !expect;
            pcb.pending_sigs.should_wake = expect;
        }
        debug!(
            "[sys_sigsuspend] tid: {}, suspend with mask: {:?}, old mask: {:?}, invoke_signal: {:?}",
            task.tid(),
            pcb.sig_mask(),
            old_mask,
            invoke_signal,
        );
        drop(pcb);
        assert_no_lock!();
        suspend_now().await;
        // fixme: the signal mask is not restored correctly
        // *task.pcb().sig_mask_mut() = old_mask;
        Err(Errno::EINTR)
    }
}
