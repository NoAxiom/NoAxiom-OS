use alloc::{string::ToString, vec::Vec};
use core::time::Duration;

use arch::TrapArgs;
use config::task::BUSYBOX;

use super::{Syscall, SyscallResult};
use crate::{
    fs::path::Path,
    include::{
        futex::{
            FUTEX_BITSET_MATCH_ANY, FUTEX_CLOCK_REALTIME, FUTEX_CMD_MASK, FUTEX_REQUEUE,
            FUTEX_WAIT, FUTEX_WAIT_BITSET, FUTEX_WAKE, FUTEX_WAKE_BITSET,
        },
        process::{
            robust_list::RobustList,
            rusage::{Rusage, RUSAGE_SELF},
            CloneArgs, CloneFlags, PidSel, WaitOption,
        },
        result::Errno,
        time::TimeSpec,
    },
    mm::user_ptr::UserPtr,
    return_errno,
    sched::{
        spawn::spawn_utask,
        utils::{intable, yield_now},
    },
    signal::{
        sig_detail::{SigDetail, SigKillDetail},
        sig_info::{SigCode, SigInfo},
        sig_set::SigSet,
    },
    task::{
        exit::ExitCode,
        futex::FutexFuture,
        manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
    },
    time::timeout::TimeLimitedFuture,
};

impl Syscall<'_> {
    /// exit current task by marking it as zombie
    pub fn sys_exit(&self, exit_code: i32) -> SyscallResult {
        self.task.terminate(ExitCode::new(exit_code));
        Ok(0)
    }

    /// exit group
    pub fn sys_exit_group(&self, exit_code: i32) -> SyscallResult {
        let task = self.task;
        let exit_code = ExitCode::new(exit_code);
        task.terminate_group(exit_code);
        task.terminate(exit_code);
        Ok(0)
    }

    /// clone current task
    pub async fn sys_clone(
        &self,
        flags: usize,
        stack: usize,
        ptid: usize,
        tls: usize,
        ctid: usize,
    ) -> SyscallResult {
        let flags = CloneFlags::from_bits(flags & !0xff).unwrap();
        let new_task = self.task.fork(flags);
        let new_tid = new_task.tid();
        let new_cx = new_task.trap_context_mut();
        debug!(
            "[sys_fork] flags: {:?} stack: {:#x} ptid: {:#x} tls: {:#x} ctid: {:#x}",
            flags, stack, ptid, tls, ctid
        );
        use TrapArgs::*;
        if stack != 0 {
            new_cx[SP] = stack;
        }
        if flags.contains(CloneFlags::SETTLS) {
            new_cx[TLS] = tls;
        }
        if flags.contains(CloneFlags::PARENT_SETTID) {
            let ptid = UserPtr::<usize>::new(ptid);
            ptid.write(new_tid).await?;
        }
        if flags.contains(CloneFlags::CHILD_SETTID) {
            new_task.tcb_mut().set_child_tid = Some(ctid);
        }
        if flags.contains(CloneFlags::CHILD_CLEARTID) {
            new_task.tcb_mut().clear_child_tid = Some(ctid);
        }
        new_cx[RES] = 0;
        trace!("[sys_fork] new task context: {:?}", new_cx);
        info!(
            "[sys_fork] parent: TID{} child: TID{}",
            self.task.tid(),
            new_task.tid(),
        );
        spawn_utask(new_task);
        // TASK_MANAGER.get_init_proc().print_child_tree();
        Ok(new_tid as isize)
    }

    /// clone3
    pub async fn sys_clone3(&self, cl_args: usize, _size: usize) -> SyscallResult {
        let cl_args = UserPtr::<CloneArgs>::new(cl_args);
        let cl_args = cl_args.read().await?;
        warn!("[sys_clone3] cl_args: {:#x?}", cl_args);
        let flags = cl_args.flags as usize;
        let stack = cl_args.stack as usize + cl_args.stack_size as usize - 16;
        let ptid = cl_args.parent_tid as usize;
        let ctid = cl_args.child_tid as usize;
        let tls = cl_args.tls as usize;
        self.sys_clone(flags, stack, ptid, tls, ctid).await
    }

    /// execve syscall impl
    /// execute a new program, replacing the current process image
    pub async fn sys_execve(&self, path: usize, argv: usize, envp: usize) -> SyscallResult {
        let mut path = UserPtr::new(path).get_cstr()?;
        let mut args = Vec::new();
        let mut envs = Vec::new();
        debug!("[sys_execve] path: {:?}", path);

        // args and envs init
        if path.ends_with(".sh") {
            info!("[execve] executing .sh script, path: {:?}", path);
            path = BUSYBOX.to_string();
            args.push(format!("busybox"));
            args.push(format!("sh"));
        }
        // else if path.ends_with("sleep") {
        //     info!("[execve] executing sleep, path: {:?}", path);
        //     path = BUSYBOX.to_string();
        //     args.push(format!("busybox"));
        // }

        #[cfg(feature = "debug_sig")]
        {
            use crate::utils::loghook::{logoff, logon};
            if path.ends_with("logon") {
                logon();
                return Ok(0);
            } else if path.ends_with("logoff") {
                logoff();
                return Ok(0);
            }
        }

        let file_path = Path::from_string(path, self.task)?;
        // append args and envs from user provided
        args.append(&mut UserPtr::<UserPtr<u8>>::new(argv).get_string_vec().await?);
        envs.append(&mut UserPtr::<UserPtr<u8>>::new(envp).get_string_vec().await?);

        info!(
            "[sys_exec] TID{}, path: {:?}, argv: {:#x}, envp: {:#x}, arg: {:?}, env: {:?}",
            self.task.tid(),
            file_path,
            argv,
            envp,
            args,
            envs,
        );
        self.task.execve(file_path, args, envs).await?;

        // On success, execve() does not return, on error -1 is returned, and errno is
        // set to indicate the error.
        Ok(0 as isize)
    }

    pub async fn sys_wait4(&self, pid: isize, status_addr: usize, options: usize) -> SyscallResult {
        trace!(
            "[sys_wait4] pid: {:?}, status_addr: {:?}, options: {:?}",
            pid,
            status_addr,
            options
        );
        let wait_option = WaitOption::from_bits(options as i32).ok_or(Errno::EINVAL)?;
        let status: UserPtr<i32> = UserPtr::new(status_addr);

        // pid type
        // -1: all children, >0: specific pid, other: group unimplemented
        let pid_type = match pid {
            -1 => PidSel::Task(None),
            0 => PidSel::Group(None),
            pid if pid > 0 => PidSel::Task(Some(pid as usize)),
            pid => PidSel::Group(Some(pid as usize)),
        };

        // wait for child exit
        let (exit_code, tid) = intable(
            self.task,
            self.task.wait_child(pid_type, wait_option),
            Some(SigSet::SIGCHLD),
        )
        .await??;
        if status.is_non_null() {
            trace!(
                "[sys_wait4]: write exit_code at status_addr = {:#x}, value: ",
                status.va_addr().raw(),
            );
            status.write(exit_code.inner()).await?;
            trace!("[sys_wait4]: write exit code {:#x}", exit_code.inner());
        }
        Ok(tid as isize)
    }

    pub fn sys_gettid(&self) -> SyscallResult {
        Ok(self.task.tid() as isize)
    }

    pub fn sys_getpid(&self) -> SyscallResult {
        Ok(self.task.tgid() as isize)
    }

    pub fn sys_getppid(&self) -> SyscallResult {
        let parent_process = self.task.pcb().parent.clone();
        match parent_process {
            None => Err(Errno::ESRCH),
            Some(parent_process) => Ok(parent_process.upgrade().unwrap().tid() as isize),
        }
    }

    pub fn sys_set_tid_address(&self, tidptr: usize) -> SyscallResult {
        let task = self.task;
        task.tcb_mut().clear_child_tid = Some(tidptr);
        Ok(task.tid() as isize)
    }

    pub fn sys_getpgid(&self, pid: usize) -> SyscallResult {
        let target_task = if pid == 0 {
            self.task.clone()
        } else {
            TASK_MANAGER.get(pid).ok_or(Errno::ESRCH)?
        };
        let pgid = target_task.get_pgid();
        info!("[sys_getpgid] tid: {}, pgid: {}", target_task.tid(), pgid);
        Ok(pgid as isize)
    }

    pub fn sys_setpgid(&self, pid: usize, pgid: usize) -> SyscallResult {
        if (pgid as isize) < 0 {
            return_errno!(Errno::EINVAL);
        }

        // If pid is zero, then the process ID of the calling process is used
        let target_task = if pid == 0 {
            self.task.clone()
        } else {
            TASK_MANAGER.get(pid).ok_or(Errno::ESRCH)?
        };

        // If pgid is zero, then the PGID of the process specified by pid is
        // made the same as its process ID.
        if pgid == 0 {
            PROCESS_GROUP_MANAGER
                .lock()
                .create_new_group_by(&target_task);
        } else {
            PROCESS_GROUP_MANAGER.lock().modify_pgid(&target_task, pgid);
        }
        Ok(0)
    }

    pub fn sys_set_robust_list(&self, head: usize, len: usize) -> SyscallResult {
        info!("[sys_set_robust_list] head {:#x}, len {:#x}", head, len);
        if len != RobustList::HEAD_SIZE {
            error!("robust list head len mismatch: len={}", len);
            return_errno!(Errno::EINVAL);
        }
        let mut pcb = self.task.pcb();
        pcb.robust_list.head = head;
        pcb.robust_list.len = len;
        Ok(0)
    }

    pub async fn sys_get_robust_list(
        &self,
        pid: usize,
        head_ptr: usize,
        len_ptr: usize,
    ) -> SyscallResult {
        warn!("[sys_get_robust_list]");
        info!(
            "[sys_get_robust_list] pid {:?} head {:#x}, len {:#x}",
            pid, head_ptr as usize, len_ptr as usize
        );
        let tid = if pid == 0 { self.task.tid() } else { pid };
        let Some(task) = TASK_MANAGER.get(tid) else {
            return_errno!(Errno::ESRCH);
        };
        let robust_list = task.pcb().robust_list;
        let head_ptr = UserPtr::<usize>::new(head_ptr);
        let len_ptr = UserPtr::<usize>::new(len_ptr);
        head_ptr.write(robust_list.head).await?;
        len_ptr.write(robust_list.len).await?;
        Ok(0)
    }

    pub async fn sys_futex(
        &self,
        uaddr: usize,
        futex_op: usize,
        val: u32,
        val2: usize,
        uaddr2: usize,
        val3: u32,
    ) -> SyscallResult {
        let option = futex_op & FUTEX_CMD_MASK;
        if (futex_op & FUTEX_CLOCK_REALTIME) != 0
            && option != FUTEX_WAIT
            && option != FUTEX_WAIT_BITSET
        {
            return_errno!(
                Errno::EPERM,
                "[FUTEX ERROR] uaddr {:#x}, futex_op {:#x}, option {:#x}, val {:#x}, val2 {:#x}, uaddr2 {:#x}, val3 {:#x}",
                uaddr, futex_op, option, val, val2, uaddr2, val3,
            );
        }
        info!(
            "[sys_futex] uaddr {:#x}, futex_op {:#x}, val {:#x}, val2 {:#x}, uaddr2 {:#x}, val3 {:#x}",
            uaddr, option, val, val2, uaddr2, val3
        );

        let task = self.task;
        match option {
            FUTEX_WAIT | FUTEX_WAIT_BITSET => {
                let bitset: u32 = match option {
                    FUTEX_WAIT_BITSET => val3,
                    FUTEX_WAIT => FUTEX_BITSET_MATCH_ANY,
                    _ => unreachable!(),
                };
                let futex_word = UserPtr::<u32>::new(uaddr);
                let pa = futex_word.translate_pa().await?;
                let timeout = match val2 {
                    0 => None,
                    val2 => {
                        let val2 = UserPtr::<TimeSpec>::new(val2);
                        let time_spec = val2.read().await?;
                        let limit_time = Duration::from(time_spec);
                        info!("[sys_futex]: timeout {:?}", limit_time);
                        Some(limit_time)
                    }
                };
                let res = TimeLimitedFuture::new(FutexFuture::new(uaddr, pa, val, bitset), timeout).await
                // intable(
                //     self.task,
                //     TimeLimitedFuture::new(FutexFuture::new(uaddr, pa, val, bitset), timeout),
                //     None,
                // )
                // .await?
                .map_timeout(Err(Errno::ETIMEDOUT))?;
                Ok(res)
            }
            FUTEX_WAKE | FUTEX_WAKE_BITSET => {
                let bitset = match option {
                    FUTEX_WAKE_BITSET => val3,
                    FUTEX_WAKE => FUTEX_BITSET_MATCH_ANY,
                    _ => unreachable!(),
                };
                if bitset == 0 {
                    return_errno!(Errno::EINVAL, "[sys_futex] bitset is 0");
                }
                let futex_word = UserPtr::<u32>::new(uaddr);
                let pa = futex_word.translate_pa().await?;
                let res = task.futex().wake_waiter(pa, val, bitset);
                info!(
                    "[sys_futex] futex wake, uaddr = {:#x}, val = {}, res: {:?}",
                    uaddr, val, res
                );
                yield_now().await;
                Ok(res as isize)
            }
            FUTEX_REQUEUE => {
                let old_word = UserPtr::<u32>::new(uaddr);
                let new_word = UserPtr::<u32>::new(uaddr2);
                let old_pa = old_word.translate_pa().await?;
                let new_pa = new_word.translate_pa().await?;
                warn!(
                    "[sys_futex] futex requeue: uaddr={:#x}, uaddr2={:#x}, val={}, val2={}",
                    uaddr, uaddr2, val, val2
                );
                Ok(task.futex().requeue(old_pa, new_pa, val, val2 as u32) as isize)
            }
            _ => return_errno!(Errno::EINVAL),
        }
    }

    pub fn sys_setsid(&self) -> SyscallResult {
        let task = self.task;
        warn!(
            "[sys_setsid] tid: {}, pid: {}, pgid: {}, using incorrect implementation",
            task.tid(),
            task.pid(),
            task.get_pgid()
        );
        Ok(task.tgid() as isize)
        // fixme: this impl is incorrect!!!
        // if task.is_group_leader() && task.tgid() != task.get_pgid() {
        //     PROCESS_GROUP_MANAGER.lock().modify_pgid(task, task.tid());
        //     Ok(task.tgid() as isize)
        // } else {
        //     Err(Errno::EPERM)
        // }
    }

    pub fn sys_tkill(&self, tid: usize, signal: i32) -> SyscallResult {
        if signal == 0 {
            return Ok(0);
        }
        trace!("[sys_tkill] tid: {}, signal num: {}", tid, signal);
        let task = TASK_MANAGER.get(tid).ok_or(Errno::ESRCH)?;
        let pid = task.tgid() as _;
        task.recv_siginfo(
            SigInfo {
                signo: signal,
                code: SigCode::TKill,
                errno: 0,
                detail: SigDetail::Kill(SigKillDetail { pid }),
            },
            true,
        );
        Ok(0)
    }

    pub fn sys_tgkill(&self, tgid: usize, tid: usize, signal: i32) -> SyscallResult {
        if signal == 0 {
            return Ok(0);
        }
        trace!(
            "[sys_tgkill] tgid: {}, tid: {}, signal num: {}",
            tgid,
            tid,
            signal
        );
        match tgid as isize {
            -1 => self.sys_tkill(tid, signal),
            _ => {
                let task = TASK_MANAGER.get(tid).ok_or(Errno::ESRCH)?;
                if task.tgid() != tgid {
                    return Err(Errno::ESRCH);
                }
                let cur_pid = self.task.tgid();
                task.recv_siginfo(
                    SigInfo {
                        signo: signal,
                        code: SigCode::TKill,
                        errno: 0,
                        detail: SigDetail::Kill(SigKillDetail { pid: cur_pid }),
                    },
                    true,
                );
                Ok(0)
            }
        }
    }

    pub async fn sys_getrusage(&self, who: isize, usage: usize) -> SyscallResult {
        if who != RUSAGE_SELF {
            return_errno!(Errno::EINVAL);
        }
        let usage = UserPtr::<Rusage>::from(usage);
        let mut rusage = Rusage::new();
        let tgroup = self.task.thread_group();
        let mut utime = Duration::ZERO;
        let mut stime = Duration::ZERO;
        let mut start_time = Duration::ZERO;
        for (_, thread) in tgroup.0.iter() {
            if let Some(thread) = thread.upgrade() {
                utime += thread.time_stat().utime();
                stime += thread.time_stat().stime();
                if start_time.is_zero() {
                    start_time = thread.time_stat().create_time();
                }
            };
        }
        rusage.ru_stime = stime.into();
        rusage.ru_utime = utime.into();
        usage.write(rusage).await?;
        Ok(0)
    }
}
