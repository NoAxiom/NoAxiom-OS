use alloc::vec::Vec;
use core::time::Duration;

use arch::TrapArgs;
use config::fs::ROOT_NAME;

use super::{Syscall, SyscallResult};
use crate::{
    fs::path::Path,
    include::{
        futex::{FUTEX_CLOCK_REALTIME, FUTEX_CMD_MASK, FUTEX_REQUEUE, FUTEX_WAIT, FUTEX_WAKE},
        process::{robust_list::RobustList, CloneFlags, PidSel, WaitOption},
        result::Errno,
    },
    mm::user_ptr::UserPtr,
    return_errno,
    sched::{
        spawn::spawn_utask,
        utils::{intable, yield_now},
    },
    task::{
        exit::ExitCode,
        futex::FutexFuture,
        manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
    },
    time::{time_spec::TimeSpec, timeout::TimeLimitedFuture},
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
        #[allow(unused_mut)] mut tls: usize,
        #[allow(unused_mut)] mut ctid: usize,
    ) -> SyscallResult {
        /*
           On x86-32, and several other common architectures (including
           score, ARM, ARM 64, PA-RISC, arc, Power PC, xtensa, and MIPS), the
           order of the last two arguments is reversed.
           And so on loongarch64.
           ref1: https://www.man7.org/linux/man-pages/man2/clone.2.html#VERSIONS
           ref2: https://inbox.vuxu.org/musl/1a5a097f.12d7.1794a6de3a8.Coremail.zhaixiaojuan%40loongson.cn/t/
           sys_clone(u64 flags, u64 ustack_base, u64 parent_tidptr, u64 child_tidptr, u64 tls)
        */
        #[cfg(target_arch = "loongarch64")]
        core::mem::swap(&mut tls, &mut ctid);

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
            let ctid = UserPtr::<usize>::new(ctid);
            ctid.write(new_tid).await?;
        }
        if flags.contains(CloneFlags::CHILD_CLEARTID) {
            new_task.set_clear_tid_address(ctid);
        }
        new_cx[RES] = 0;
        trace!("[sys_fork] new task context: {:?}", new_cx);
        spawn_utask(new_task);
        TASK_MANAGER.get_init_proc().print_child_tree();
        Ok(new_tid as isize)
    }

    /// execve syscall impl
    /// execute a new program, replacing the current process image
    pub async fn sys_execve(&self, path: usize, argv: usize, envp: usize) -> SyscallResult {
        let mut path = UserPtr::new(path).get_cstr();
        let mut args = Vec::new();
        let mut envs = Vec::new();
        debug!("[sys_execve] path: {:?}", path);

        // args and envs init
        if path.contains(".sh") {
            info!("[execve] executing .sh script, path: {:?}", path);
            path = format!("{ROOT_NAME}/busybox");
            args.push(format!("busybox"));
            args.push(format!("sh"));
        } else if path.ends_with("ls") {
            info!("[execve] executing ls, path: {:?}", path);
            path = format!("busybox");
            args.push(format!("busybox"));
        } else if path.ends_with("sleep") {
            info!("[execve] executing sleep, path: {:?}", path);
            path = format!("busybox");
            args.push(format!("busybox"));
        }

        let file_path = Path::from_string(path, self.task)?;
        // append args and envs from user provided
        args.append(&mut UserPtr::<UserPtr<u8>>::new(argv).get_string_vec().await?);
        envs.append(&mut UserPtr::<UserPtr<u8>>::new(envp).get_string_vec().await?);

        info!(
            "[sys_exec] path: {:?} argv: {:#x}, envp: {:#x}, arg: {:?}, env: {:?}",
            file_path, argv, envp, args, envs,
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
        let (exit_code, tid) =
            intable(self.task, self.task.wait_child(pid_type, wait_option)).await??;
        if status.is_not_null() {
            trace!(
                "[sys_wait4]: write exit_code at status_addr = {:#x}",
                status.va_addr().raw(),
            );
            status.write(ExitCode::new(exit_code).inner()).await?;
            trace!("[sys_wait4]: write exit code {:#x}", exit_code);
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
        task.set_clear_tid_address(tidptr);
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
            return Err(Errno::EINVAL);
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
        if futex_op & FUTEX_CLOCK_REALTIME != 0 && option != FUTEX_WAIT {
            return_errno!(Errno::EPERM);
        }
        info!(
            "[sys_futex] uaddr {:#x}, futex_op {:#x}, val {:#x}, val2 {:#x}, uaddr2 {:#x}, val3 {:#x}",
            uaddr, option, val, val2, uaddr2, val3
        );

        let task = self.task;
        match option {
            FUTEX_WAIT => {
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
                let res = TimeLimitedFuture::new(FutexFuture::new(uaddr, pa, val), timeout)
                    .await
                    .map_timeout(Err(Errno::ETIMEDOUT))?;
                Ok(res)
            }
            FUTEX_WAKE => {
                let futex_word = UserPtr::<u32>::new(uaddr);
                let pa = futex_word.translate_pa().await?;
                let res = task.futex().wake_waiter(pa, val);
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
            _ => Err(Errno::EINVAL),
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
}
