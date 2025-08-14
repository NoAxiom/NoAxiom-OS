use alloc::{string::ToString, vec::Vec};
use core::time::Duration;

use config::task::BUSYBOX;

use super::{Syscall, SyscallResult};
use crate::{
    constant::fs::AT_FDCWD,
    fs::path::get_dentry,
    include::{
        fs::{FileFlags, SearchFlags},
        futex::{FutexFlags, FutexOps, FUTEX_BITSET_MATCH_ANY},
        process::{
            robust_list::RobustList,
            rusage::{Rusage, RUSAGE_SELF},
            CloneArgs, PidSel, WaitOption,
        },
        result::Errno,
        time::TimeSpec,
    },
    mm::user_ptr::UserPtr,
    return_errno,
    sched::utils::yield_now,
    signal::{
        interruptable::interruptable,
        sig_detail::{SigDetail, SigKillDetail},
        sig_info::{SigCode, SigInfo},
        sig_set::SigMask,
        signal::Signal,
    },
    task::{
        exit::ExitCode,
        futex::{FutexAddr, FutexFuture, FUTEX_SHARED_QUEUE},
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

    pub async fn sys_clone(&self, args: &[usize; 6]) -> SyscallResult {
        /*
         * On x86-32, and several other common architectures (including
         * score, ARM, ARM 64, PA-RISC, arc, Power PC, xtensa, and MIPS), the
         * order of the last two arguments is reversed.
         * And so on loongarch64.
         * ref1: https://www.man7.org/linux/man-pages/man2/clone.2.html#VERSIONS
         * ref2: https://inbox.vuxu.org/musl/1a5a097f.12d7.1794a6de3a8.Coremail.zhaixiaojuan%40loongson.cn/t/
         * sys_clone(u64 flags, u64 ustack_base, u64 parent_tidptr, u64 child_tidptr,
         * u64 tls)
         */
        #[cfg(target_arch = "loongarch64")]
        let cl_args = CloneArgs::from_legacy(args[0], args[1], args[2], args[4], args[3]);
        #[cfg(target_arch = "riscv64")]
        let cl_args = CloneArgs::from_legacy(args[0], args[1], args[2], args[3], args[4]);
        self.task.do_fork(cl_args).await
    }

    /// clone3
    pub async fn sys_clone3(&self, cl_args: usize, size: usize) -> SyscallResult {
        let cl_args = UserPtr::<CloneArgs>::new(cl_args);
        const MINIMAL_CLARGS_SIZE: usize = core::mem::size_of::<u64>() * 8;
        if size < MINIMAL_CLARGS_SIZE {
            return Err(Errno::EINVAL);
        }
        if size > core::mem::size_of::<CloneArgs>() {
            return Err(Errno::EFAULT);
        }
        let cl_args = cl_args.read().await?;
        self.task.do_fork(cl_args).await
    }

    /// execve syscall impl
    /// execute a new program, replacing the current process image
    pub async fn sys_execve(&self, path: usize, argv: usize, envp: usize) -> SyscallResult {
        let mut path = UserPtr::new(path).get_cstr()?;
        let mut args = Vec::new();
        let mut envs = Vec::new();
        debug!("[sys_execve] path: {:?}", path);

        // args and envs init
        // FIXME: MENTION that maybe can use the task.exe path or the path from function
        // parameters directly
        //
        // like: args.push(path)
        if path.ends_with(".sh") {
            info!("[execve] executing .sh script, path: {:?}", path);
            path = BUSYBOX.to_string();
            args.push(format!("busybox"));
            args.push(format!("sh"));
        }

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

        let searchflags = SearchFlags::empty();
        let dentry = get_dentry(self.task, AT_FDCWD, &path, &searchflags)?;
        let file_path = dentry.path();
        let elf_file = dentry.open(&FileFlags::empty())?;

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
        self.task.execve(elf_file, args, envs).await?;

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

        if pid == i32::MIN as isize {
            return Err(Errno::ESRCH);
        }

        // pid type
        // -1: all children, >0: specific pid, other: group unimplemented
        let pid_type = match pid {
            -1 => PidSel::Task(None),
            0 => PidSel::Group(None),
            pid if pid > 0 => PidSel::Task(Some(pid as usize)),
            pid => PidSel::Group(Some(pid as usize)),
        };

        // wait for child exit
        let (exit_code, tid) = interruptable(
            self.task,
            self.task.wait_child(pid_type, wait_option),
            None,
            Some(SigMask::SIGCHLD),
        )
        .await??;
        self.task.pcb().signals.remove_sigchld();
        if status.is_non_null() {
            debug!(
                "[sys_wait4]: write exit_code at status_addr = {:#x}, value: {:#x}",
                status.va_addr().raw(),
                exit_code.inner()
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
        let option = FutexOps::from_repr(futex_op & FutexFlags::FUTEX_CMD_MASK.bits())
            .ok_or(Errno::EINVAL)?;
        let flags = FutexFlags::from_bits_retain(futex_op & FutexFlags::FUTEX_FLAG_MASK.bits());
        if flags.is_clock_realtime() && option.is_futex_wake() {
            return Err(Errno::EPERM);
        }
        info!(
            "[sys_futex] uaddr {:#x}, option {:?}, flags: {:?}, val {:#x}, val2 {:#x}, uaddr2 {:#x}, val3 {:#x}",
            uaddr, option, flags, val, val2, uaddr2, val3
        );

        let task = self.task;
        match option {
            FutexOps::FutexWait | FutexOps::FutexWaitBitset => {
                let bitset: u32 = match option {
                    FutexOps::FutexWaitBitset => val3,
                    FutexOps::FutexWait => FUTEX_BITSET_MATCH_ANY,
                    _ => unreachable!(),
                };
                let faddr = FutexAddr::new(uaddr, flags).await?;
                let timeout = match val2 {
                    0 => None,
                    val2 => {
                        let val2 = UserPtr::<TimeSpec>::new(val2);
                        let time_spec = val2.read().await?;
                        if !time_spec.is_valid() {
                            error!("[sys_futex]: invalid timespec");
                            return_errno!(Errno::EINVAL);
                        }
                        let limit_time = Duration::from(time_spec);
                        info!("[sys_futex]: timeout {:?}", limit_time);
                        Some(limit_time)
                    }
                };
                info!(
                    "[sys_futex] futex wait, uaddr = {:#x}, faddr = {:x?}, val = {}, bitset = {:#x}, timeout = {:?}",
                    uaddr, faddr, val, bitset, timeout
                );
                let res = match faddr {
                    FutexAddr::Private(faddr) => interruptable(
                        self.task,
                        TimeLimitedFuture::new(
                            FutexFuture::new(uaddr, faddr, val, bitset, task.futex_ref()),
                            timeout,
                        ),
                        None,
                        None,
                    )
                    .await?
                    .map_timeout(Err(Errno::ETIMEDOUT))?,
                    FutexAddr::Shared(faddr) => interruptable(
                        self.task,
                        TimeLimitedFuture::new(
                            FutexFuture::new(uaddr, faddr, val, bitset, &FUTEX_SHARED_QUEUE),
                            timeout,
                        ),
                        None,
                        None,
                    )
                    .await?
                    .map_timeout(Err(Errno::ETIMEDOUT))?,
                };
                Ok(res)
            }
            FutexOps::FutexWake | FutexOps::FutexWakeBitset => {
                let bitset = match option {
                    FutexOps::FutexWakeBitset => val3,
                    FutexOps::FutexWake => FUTEX_BITSET_MATCH_ANY,
                    _ => unreachable!(),
                };
                if bitset == 0 {
                    return_errno!(Errno::EINVAL, "[sys_futex] bitset is 0");
                }
                let addr = FutexAddr::new(uaddr, flags).await?;
                let res = match addr {
                    FutexAddr::Private(addr) => task.futex().wake_waiter(addr, val, bitset),
                    FutexAddr::Shared(addr) => {
                        FUTEX_SHARED_QUEUE.lock().wake_waiter(addr, val, bitset)
                    }
                };
                info!(
                    "[sys_futex] futex wake, uaddr = {:#x}, faddr = {:x?}, val = {}, res: {:?}",
                    uaddr, addr, val, res
                );
                yield_now().await;
                Ok(res as isize)
            }
            FutexOps::FutexRequeue => {
                warn!(
                    "[sys_futex] futex requeue: uaddr={:#x}, uaddr2={:#x}, val={}, val2={}",
                    uaddr, uaddr2, val, val2
                );
                match flags.is_private() {
                    true => {
                        let old_pa = FutexAddr::new_private(uaddr);
                        let new_pa = FutexAddr::new_private(uaddr2);
                        Ok(task.futex().requeue(old_pa, new_pa, val, val2 as u32) as isize)
                    }
                    false => {
                        let old_pa = FutexAddr::new_shared(uaddr).await?;
                        let new_pa = FutexAddr::new_shared(uaddr2).await?;
                        Ok(FUTEX_SHARED_QUEUE
                            .lock()
                            .requeue(old_pa, new_pa, val, val2 as u32)
                            as isize)
                    }
                }
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

    pub fn sys_tkill(&self, tid: usize, signo: usize) -> SyscallResult {
        if signo == 0 {
            error!("[sys_tkill] signo is 0, no signal to send");
            return Ok(0);
        }
        let signal = Signal::try_from(signo)?;
        self.__sys_tkill(tid, signal)
    }

    pub fn __sys_tkill(&self, tid: usize, signal: Signal) -> SyscallResult {
        info!("[sys_tkill] tid: {}, signal: {:?}", tid, signal);
        let task = TASK_MANAGER.get(tid).ok_or(Errno::ESRCH)?;
        let pid = task.tgid() as _;
        task.recv_siginfo(
            SigInfo {
                signal,
                code: SigCode::TKill,
                errno: 0,
                detail: SigDetail::Kill(SigKillDetail { pid }),
            },
            true,
        );
        Ok(0)
    }

    pub fn sys_tgkill(&self, tgid: usize, tid: usize, signo: usize) -> SyscallResult {
        if signo == 0 {
            error!("[sys_tkill] signo is 0, no signal to send");
            return Ok(0);
        }
        let signal = Signal::try_from(signo)?;
        trace!(
            "[sys_tgkill] tgid: {}, tid: {}, signal: {:?}",
            tgid,
            tid,
            signal
        );
        match tgid as isize {
            -1 => self.__sys_tkill(tid, signal),
            _ => {
                let task = TASK_MANAGER.get(tid).ok_or(Errno::ESRCH)?;
                if task.tgid() != tgid {
                    return Err(Errno::ESRCH);
                }
                let cur_pid = self.task.tgid();
                task.recv_siginfo(
                    SigInfo {
                        signal,
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

    pub fn sys_setuid(&self, uid: u32) -> SyscallResult {
        info!("[sys_setuid] set uid to {}", uid);
        let mut user_id = self.task.user_id();
        if user_id.euid() == 0 {
            warn!(
                "[sys_setuid] task {} is root, set uid to {}",
                self.task.tid(),
                uid
            );
            user_id.set_uid(uid);
            user_id.set_euid(uid);
            user_id.set_suid(uid);
            user_id.set_fsuid(uid);
        } else {
            if uid != user_id.uid() && uid != user_id.suid() {
                warn!(
                    "[sys_setuid] task {} is not root, set uid to {}",
                    self.task.tid(),
                    uid
                );
                return Err(Errno::EPERM);
            } else {
                warn!("[sys_setuid] task {} set uid to {}", self.task.tid(), uid);
                user_id.set_euid(uid);
                user_id.set_fsuid(uid);
            }
        }
        Ok(0)
    }

    pub fn sys_setgid(&self, gid: u32) -> SyscallResult {
        info!("[sys_setgid] set gid to {}", gid);
        let mut user_id = self.task.user_id();
        if user_id.euid() == 0 {
            warn!(
                "[sys_setgid] task {} is root, set gid to {}",
                self.task.tid(),
                gid
            );
            user_id.set_gid(gid);
            user_id.set_egid(gid);
            user_id.set_sgid(gid);
            user_id.set_fsgid(gid);
        } else {
            if gid != user_id.gid() && gid != user_id.sgid() {
                warn!(
                    "[sys_setgid] task {} is not root, set gid to {}",
                    self.task.tid(),
                    gid
                );
                return Err(Errno::EPERM);
            } else {
                warn!("[sys_setgid] task {} set gid to {}", self.task.tid(), gid);
                user_id.set_egid(gid);
                user_id.set_fsgid(gid);
            }
        }
        Ok(0)
    }

    pub fn sys_getuid(&self) -> SyscallResult {
        Ok(self.task.user_id().uid() as isize)
    }

    pub fn sys_geteuid(&self) -> SyscallResult {
        Ok(self.task.user_id().euid() as isize)
    }

    pub fn sys_getgid(&self) -> SyscallResult {
        Ok(self.task.user_id().gid() as isize)
    }

    pub fn sys_getegid(&self) -> SyscallResult {
        Ok(self.task.user_id().egid() as isize)
    }

    pub async fn sys_setgroups(&self, size: usize, list: usize) -> SyscallResult {
        info!("[sys_setgroups] size: {}, list: {:x}", size, list);
        const NGROUPS_MAX: usize = 32; // todo: 65536
        if size > NGROUPS_MAX {
            return Err(Errno::EINVAL);
        }
        if self.task.user_id().euid() != 0 {
            return Err(Errno::EPERM);
        }

        let groups = UserPtr::<u32>::new(list);
        let groups = groups.as_slice_const_checked(size).await?;

        let mut sup_groups = self.task.sup_groups();
        if size == 0 {
            sup_groups.clear();
        } else {
            sup_groups.extend(groups);
        }

        Ok(0)
    }

    /// Get the supplementary groups of the current self.task.
    pub async fn sys_getgroups(&self, size: usize, list: usize) -> SyscallResult {
        info!("[sys_getgroups] size: {}, list: {:x}", size, list);
        const NGROUPS_MAX: usize = 32; // todo: 65536
        if size > NGROUPS_MAX {
            return Err(Errno::EINVAL);
        }

        let sup_groups = self.task.sup_groups();
        let len = sup_groups.len();
        if size == 0 {
            return Ok(len as isize);
        }
        if size < len {
            return Err(Errno::EINVAL);
        }

        let groups = UserPtr::<u32>::new(list);
        let groups = groups.as_slice_mut_checked(size).await?;
        if list != 0 {
            groups[..len].copy_from_slice(&sup_groups[..len]);
        }
        Ok(len as isize)
    }

    /// ref: RocketOS
    pub fn sys_setreuid(&self, ruid: i32, euid: i32) -> SyscallResult {
        info!("[sys_setreuid] ruid: {}, euid: {}", ruid, euid);
        let mut user_id = self.task.user_id();
        let origin_uid = user_id.uid() as i32;
        let origin_euid = user_id.euid() as i32;
        let origin_suid = user_id.suid() as i32;
        if user_id.euid() == 0 {
            warn!("[sys_setreuid] is root, set ruid: {}, euid: {}", ruid, euid);
            if ruid != -1 {
                user_id.set_uid(ruid as u32);
            }
            if euid != -1 {
                user_id.set_euid(euid as u32);
                user_id.set_fsuid(euid as u32);
            }
        } else {
            if ruid != -1 {
                if ruid != origin_uid as i32 && ruid != origin_euid as i32 {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setreuid] is not root, set ruid: {}", ruid);
                user_id.set_uid(ruid as u32);
            }
            if euid != -1 {
                if euid != origin_uid as i32
                    && euid != origin_euid as i32
                    && euid != origin_suid as i32
                {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setreuid] is not root, set euid: {}", euid);
                user_id.set_euid(euid as u32);
                user_id.set_fsuid(euid as u32);
            }
        }
        if ruid != -1 || (euid != -1 && euid != origin_uid as i32) {
            let tmp_euid = user_id.euid();
            user_id.set_suid(tmp_euid as u32);
        }
        Ok(0)
    }

    pub fn sys_setregid(&self, rgid: i32, egid: i32) -> SyscallResult {
        info!("[sys_setregid] rgid: {}, egid: {}", rgid, egid);
        let mut user_id = self.task.user_id();
        let origin_gid = user_id.gid() as i32;
        let origin_sgid = user_id.sgid() as i32;
        error!(
            "[sys_setregid] origin_gid: {}, origin_sgid: {}",
            origin_gid, origin_sgid
        );
        if user_id.euid() == 0 {
            warn!("[sys_setregid] is root, set rgid: {}, egid: {}", rgid, egid);
            if rgid != -1 {
                user_id.set_gid(rgid as u32);
            }
            if egid != -1 {
                user_id.set_egid(egid as u32);
                user_id.set_fsgid(egid as u32);
            }
        } else {
            if rgid != -1 {
                if rgid != origin_sgid as i32 && rgid != origin_gid as i32 {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setregid] is not root, set rgid: {}", rgid);
                user_id.set_gid(rgid as u32);
            }
            if egid != -1 {
                if egid != origin_gid as i32 && egid != origin_sgid as i32 {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setregid] is not root, set egid: {}", egid);
                user_id.set_egid(egid as u32);
                user_id.set_fsgid(egid as u32);
            }
        }
        if rgid != -1 || (egid != -1 && egid != origin_gid as i32) {
            let tmp_egid = user_id.egid();
            user_id.set_sgid(tmp_egid as u32);
        }
        Ok(0)
    }

    pub fn sys_setresuid(&self, ruid: i32, euid: i32, suid: i32) -> SyscallResult {
        info!(
            "[sys_setreuid] ruid: {}, euid: {}, suid: {}",
            ruid, euid, suid
        );
        let mut user_id = self.task.user_id();
        let origin_uid = user_id.uid() as i32;
        let origin_euid = user_id.euid() as i32;
        let origin_suid = user_id.suid() as i32;
        if user_id.euid() == 0 {
            warn!("[sys_setreuid] is root, set ruid: {}, euid: {}", ruid, euid);
            if ruid != -1 {
                user_id.set_uid(ruid as u32);
            }
            if euid != -1 {
                user_id.set_euid(euid as u32);
                user_id.set_fsuid(euid as u32);
            }
            if suid != -1 {
                user_id.set_suid(suid as u32);
            }
        } else {
            if ruid != -1 {
                if ruid != origin_uid as i32
                    && ruid != origin_euid as i32
                    && ruid != origin_suid as i32
                {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setreuid] is not root, set ruid: {}", ruid);
                user_id.set_uid(ruid as u32);
            }
            if euid != -1 {
                if euid != origin_uid as i32
                    && euid != origin_euid as i32
                    && euid != origin_suid as i32
                {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setreuid] is not root, set euid: {}", euid);
                user_id.set_euid(euid as u32);
                user_id.set_fsuid(euid as u32);
            }
            if suid != -1 {
                if suid != origin_uid as i32
                    && suid != origin_euid as i32
                    && suid != origin_suid as i32
                {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setreuid] is not root, set suid: {}", suid);
                user_id.set_suid(suid as u32);
            }
        }
        Ok(0)
    }

    pub fn sys_setresgid(&self, rgid: i32, egid: i32, sgid: i32) -> SyscallResult {
        info!(
            "[sys_setregid] rgid: {}, egid: {}, sgid: {}",
            rgid, egid, sgid
        );
        let mut user_id = self.task.user_id();
        let origin_gid = user_id.gid() as i32;
        let origin_egid = user_id.egid() as i32;
        let origin_sgid = user_id.sgid() as i32;
        if user_id.euid() == 0 {
            warn!("[sys_setregid] is root, set rgid: {}, egid: {}", rgid, egid);
            if rgid != -1 {
                user_id.set_gid(rgid as u32);
            }
            if egid != -1 {
                user_id.set_egid(egid as u32);
                user_id.set_fsgid(egid as u32);
            }
            if sgid != -1 {
                user_id.set_sgid(sgid as u32);
            }
        } else {
            if rgid != -1 {
                if rgid != origin_gid as i32
                    && rgid != origin_egid as i32
                    && rgid != origin_sgid as i32
                {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setregid] is not root, set rgid: {}", rgid);
                user_id.set_gid(rgid as u32);
            }
            if egid != -1 {
                if egid != origin_gid as i32
                    && egid != origin_egid as i32
                    && egid != origin_sgid as i32
                {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setregid] is not root, set egid: {}", egid);
                user_id.set_egid(egid as u32);
                user_id.set_fsgid(egid as u32);
            }
            if sgid != -1 {
                if sgid != origin_gid as i32
                    && sgid != origin_egid as i32
                    && sgid != origin_sgid as i32
                {
                    return Err(Errno::EPERM);
                }
                warn!("[sys_setregid] is not root, set sgid: {}", sgid);
                user_id.set_sgid(sgid as u32);
            }
        }
        Ok(0)
    }

    pub async fn sys_getresgid(&self, rgid: usize, egid: usize, sgid: usize) -> SyscallResult {
        info!(
            "[sys_getresgid] rgid: {:#x}, egid: {:#x}, sgid: {:#x}",
            rgid, egid, sgid
        );
        let user_id = self.task.user_id();
        let rgid = UserPtr::<u32>::new(rgid);
        let egid = UserPtr::<u32>::new(egid);
        let sgid = UserPtr::<u32>::new(sgid);
        rgid.try_write(user_id.gid()).await?;
        egid.try_write(user_id.egid()).await?;
        sgid.try_write(user_id.sgid()).await?;
        Ok(0)
    }

    pub async fn sys_getresuid(&self, ruid: usize, euid: usize, suid: usize) -> SyscallResult {
        info!(
            "[sys_getresuid] ruid: {}, euid: {}, suid: {}",
            ruid, euid, suid
        );
        let user_id = self.task.user_id();
        let ruid = UserPtr::<u32>::new(ruid);
        let euid = UserPtr::<u32>::new(euid);
        let suid = UserPtr::<u32>::new(suid);
        ruid.try_write(user_id.uid()).await?;
        euid.try_write(user_id.euid()).await?;
        suid.try_write(user_id.suid()).await?;
        Ok(0)
    }

    pub fn sys_setfsuid(&self, fsuid: i32) -> SyscallResult {
        log::info!("[sys_setfsuid] fsuid: {}", fsuid);
        let mut user_id = self.task.user_id();
        let origin_fsuid = user_id.fsuid() as i32;
        if user_id.euid() == 0 {
            log::warn!("[sys_setfsuid] is root, set fsuid to {}", fsuid);
            if fsuid != -1 {
                user_id.set_fsuid(fsuid as u32);
            }
        } else {
            if fsuid != user_id.uid() as i32
                && fsuid != user_id.euid() as i32
                && fsuid != user_id.suid() as i32
                && fsuid != user_id.fsuid() as i32
            {
                log::warn!("[sys_setfsuid] is not root, set fsuid to {}", fsuid);
                return Ok(origin_fsuid as isize);
            } else {
                log::warn!("[sys_setfsuid] set fsuid to {}", fsuid);
                if fsuid != -1 {
                    user_id.set_fsuid(fsuid as u32);
                }
            }
        }
        Ok(origin_fsuid as isize)
    }

    pub fn sys_setfsgid(&self, fsgid: i32) -> SyscallResult {
        log::info!("[sys_setfsgid] fsgid: {}", fsgid);
        let mut user_id = self.task.user_id();
        let origin_fsgid = user_id.fsgid() as i32;
        if user_id.euid() == 0 {
            if fsgid != -1 {
                log::warn!("[sys_setfsgid] is root, set fsgid to {}", fsgid);
                user_id.set_fsgid(fsgid as u32);
            }
        } else {
            if fsgid != user_id.gid() as i32
                && fsgid != user_id.egid() as i32
                && fsgid != user_id.sgid() as i32
                && fsgid != user_id.fsgid() as i32
            {
                log::warn!("[sys_setfsgid] is not root, set fsgid to {}", fsgid);
                return Ok(origin_fsgid as isize);
            } else {
                log::warn!("[sys_setfsgid] set fsgid to {}", fsgid);
                if fsgid != -1 {
                    user_id.set_fsgid(fsgid as u32);
                }
            }
        }
        Ok(origin_fsgid as isize)
    }
}
