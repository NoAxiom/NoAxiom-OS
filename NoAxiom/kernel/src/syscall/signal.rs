//! sys_sigaction
//! sys_sigprocmask
//! sys_kill
//! sys_sigreturn
//! sys_sigsuspend

use core::{future::pending, time::Duration};

use arch::{ArchTrapContext, TrapArgs};

use super::{Syscall, SyscallResult};
use crate::{
    config::task::INIT_PROCESS_ID,
    include::{process::TaskFlags, result::Errno, time::TimeSpec},
    mm::user_ptr::UserPtr,
    signal::{
        interruptable::interruptable,
        sig_action::{KSigAction, USigAction},
        sig_detail::{SigDetail, SigKillDetail},
        sig_info::{RawSigInfo, SigCode, SigInfo},
        sig_set::SigSet,
        sig_stack::SigAltStack,
        signal::Signal,
    },
    task::manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
    time::timeout::TimeLimitedFuture,
};

impl Syscall<'_> {
    pub async fn sys_sigaction(&self, signum: usize, act: usize, old_act: usize) -> SyscallResult {
        let signal = Signal::from_repr(signum)
            .ok_or(Errno::EINVAL)?
            .try_exclude_kill()?;
        let act = UserPtr::<USigAction>::new(act);
        let old_act = UserPtr::<USigAction>::new(old_act);
        let task = self.task;
        let act = act.try_read().await?;

        let mut sa = task.sa_list();
        let old = sa[signal].into_sa();
        // when detect new sig action, register it into sigaction list
        if let Some(act) = act {
            let kaction = KSigAction::from_sa(act, signal);
            sa.set_sigaction(signal, kaction);
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
        *task.sig_mask_mut() = ucontext.uc_sigmask;
        pcb.sig_stack = (ucontext.uc_stack.ss_size != 0).then_some(ucontext.uc_stack);
        cx[EPC] = ucontext.uc_mcontext.epc();
        *cx.gprs_mut() = ucontext.uc_mcontext.gprs();
        task.tcb_mut().flags.remove(TaskFlags::TIF_IN_SIGACTION);
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

        let old_sigmask = task.sig_mask();
        if let Some(mut set) = set_value {
            // sigmask shouldn't contain SIGKILL and SIGCONT
            set.remove(SigSet::SIGKILL | SigSet::SIGCONT);
            match how {
                SIGBLOCK => *task.sig_mask_mut() |= set,
                SIGUNBLOCK => *task.sig_mask_mut() &= !set,
                SIGSETMASK => *task.sig_mask_mut() = set,
                _ => return Err(Errno::EINVAL),
            };
        }
        old_set.try_write(old_sigmask).await?;
        Ok(0)
    }

    pub fn sys_kill(&self, pid: isize, signo: usize) -> SyscallResult {
        info!(
            "[sys_kill] from: {}, target: {}, signo: {}",
            self.task.tid(),
            pid,
            signo,
        );
        let signal = Signal::try_from_with_zero_as_none(signo)?;
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
                    if !self.task.check_user_permission(&task) {
                        continue;
                    }
                    if let Some(signal) = signal {
                        task.recv_siginfo(
                            SigInfo::new_detailed(
                                signal,
                                SigCode::User,
                                0,
                                SigDetail::Kill(SigKillDetail { pid: pgid }),
                            ),
                            false,
                        );
                    }
                    return Ok(0);
                }
                Err(Errno::EPERM)
            }
            -1 => {
                for (_, task) in TASK_MANAGER.0.lock().iter() {
                    let task = task.upgrade().unwrap();
                    if task.tgid() != INIT_PROCESS_ID && task.is_group_leader() {
                        if !self.task.check_user_permission(&task) {
                            continue;
                        }
                        if let Some(signal) = signal {
                            task.recv_siginfo(
                                SigInfo::new_detailed(
                                    signal,
                                    SigCode::User,
                                    0,
                                    SigDetail::Kill(SigKillDetail { pid: task.tgid() }),
                                ),
                                false,
                            );
                        }
                        return Ok(0);
                    }
                }
                Err(Errno::EPERM)
            }
            _ if pid > 0 => {
                if let Some(task) = TASK_MANAGER.get(pid as usize) {
                    if task.is_group_leader() {
                        if !self.task.check_user_permission(&task) {
                            return Err(Errno::EPERM);
                        }
                        if let Some(signal) = signal {
                            task.recv_siginfo(
                                SigInfo::new_detailed(
                                    signal,
                                    SigCode::User,
                                    0,
                                    SigDetail::Kill(SigKillDetail { pid: task.tgid() }),
                                ),
                                false,
                            );
                        }
                        return Ok(0);
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
                        if !self.task.check_user_permission(&task) {
                            return Err(Errno::EPERM);
                        }
                        if let Some(signal) = signal {
                            task.recv_siginfo(
                                SigInfo::new_detailed(
                                    signal,
                                    SigCode::User,
                                    0,
                                    SigDetail::Kill(SigKillDetail { pid: pgid }),
                                ),
                                false,
                            );
                            return Ok(0);
                        }
                    }
                }
                return Err(Errno::ESRCH);
            }
        }
    }

    pub async fn sys_sigsuspend(&self, mask: usize) -> SyscallResult {
        let mask = UserPtr::<SigSet>::from(mask).read().await?;
        self.__sys_sigsuspend(mask).await
    }

    async fn __sys_sigsuspend(&self, mask: SigSet) -> SyscallResult {
        let task = self.task;
        let mask = mask.without_kill();
        debug!("[sys_sigsuspend] tid: {}, new_mask: {:?}", task.tid(), mask);
        let _ = interruptable(task, pending::<()>(), Some(mask), None).await;
        Err(Errno::EINTR)
    }

    pub async fn sys_sigtimedwait(&self, set: usize, info: usize, timeout: usize) -> SyscallResult {
        let set = UserPtr::new(set).read().await?;
        let info = UserPtr::new(info);
        let timeout: TimeSpec = UserPtr::new(timeout).read().await?;
        if !timeout.is_valid() {
            error!("[sys_sigtimedwait]: timeout is negative");
            return Err(Errno::EINVAL);
        }
        self.__sys_sigtimedwait(set, info, timeout).await
    }

    async fn __sys_sigtimedwait(
        &self,
        set: SigSet,
        info: UserPtr<RawSigInfo>,
        timeout: TimeSpec,
    ) -> SyscallResult {
        let mask = !(set.with_kill());
        if (timeout.tv_nsec as isize) < 0 || timeout.tv_nsec >= 1_000_000_000 {
            return Err(Errno::EINVAL);
        }
        let timeout = Duration::from(timeout);
        let _ = TimeLimitedFuture::new(self.__sys_sigsuspend(mask), Some(timeout))
            .await
            .map_timeout_result(Err(Errno::EAGAIN))?;

        let si = self.task.pcb().signals.pop_with_mask(mask);
        if let Some(si) = si {
            let raw_si = si.into_raw();
            info.try_write(raw_si).await?;
            return Ok(si.signal.raw() as isize);
        } else {
            return Err(Errno::EINTR);
        }
    }

    pub async fn sys_sigpending(&self, set: usize) -> SyscallResult {
        let set = UserPtr::<SigSet>::new(set);
        let sigset = self.task.pcb().signals.pending_set;
        set.write(sigset).await?;
        Ok(0)
    }

    pub async fn sys_sigaltstack(&self, new: usize, old: usize) -> SyscallResult {
        if new == 0 {
            self.task.pcb().sig_stack = None;
            let task_old = self.task.pcb().sig_stack.take();
            let old = UserPtr::<SigAltStack>::new(old);
            if let Some(task_old) = task_old {
                old.write(task_old).await?;
            } else {
                // old.write(SigAltStack::default()).await?;
            }
            return Ok(0);
        }
        let new = UserPtr::<SigAltStack>::new(new).read().await?;
        let old = UserPtr::<SigAltStack>::new(old);
        let task_old = self.task.pcb().sig_stack.replace(new);
        debug!(
            "[sys_sigaltstack] tid: {}, new: {:?}, old: {:?}",
            self.task.tid(),
            new,
            task_old
        );
        if let Some(task_old) = task_old {
            old.write(task_old).await?;
        } else {
            // old.write(SigAltStack::default()).await?;
        }
        Ok(0)
    }
}
