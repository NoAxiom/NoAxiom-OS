use super::{Syscall, SyscallResult};
use crate::{
    fs::path::Path,
    include::{
        process::{PidSel, WaitOption},
        result::Errno,
        sched::CloneFlags,
        signal::{sig_info::SigExtraInfo, sig_set::SigMask},
    },
    mm::user_ptr::UserPtr,
    return_errno,
    sched::{spawn::spawn_utask, utils::suspend_now},
    task::manager::TASK_MANAGER,
};

impl Syscall<'_> {
    /// exit current task by marking it as zombie
    pub fn sys_exit(&mut self, exit_code: usize) -> SyscallResult {
        let exit_code = exit_code as i32;
        self.task.exit(exit_code);
        Ok(0)
    }

    pub fn sys_fork(
        &self,
        flags: usize, // 创建的标志，如SIGCHLD
        stack: usize, // 指定新进程的栈，可为0
        ptid: usize,  // 父线程ID, addr
        tls: usize,   // TLS线程本地存储描述符
        ctid: usize,  // 子线程ID, addr
    ) -> SyscallResult {
        trace!(
            "[sys_fork] flags: {:x} stack: {:?} ptid: {:?} tls: {:?} ctid: {:?}",
            flags,
            stack,
            ptid,
            tls,
            ctid
        );
        let flags = CloneFlags::from_bits(flags & !0xff).unwrap();
        let task = self.task.fork(flags);
        let trap_cx = task.trap_context_mut();
        trap_cx.set_result(0);
        if stack != 0 {
            trap_cx.set_sp(stack);
        }
        trace!("[sys_fork] new task context: {:?}", trap_cx);
        let tid = task.tid();
        spawn_utask(task);
        Ok(tid as isize)
    }

    pub async fn sys_exec(&mut self, path: usize, argv: usize, envp: usize) -> SyscallResult {
        let path = UserPtr::new(path).get_cstr();
        let path = if !path.starts_with('/') {
            let cwd = self.task.pcb().cwd.clone().from_cd(&"..");
            trace!("[sys_exec] cwd: {:?}", cwd);
            cwd.from_cd(&path)
        } else {
            Path::from(path)
        };
        info!(
            "[sys_exec] path: {:?} argv: {:#x}, envp: {:#x}",
            path, argv, envp
        );
        let args = UserPtr::<UserPtr<u8>>::new(argv).get_string_vec();
        let envs = UserPtr::<UserPtr<u8>>::new(envp).get_string_vec();
        self.task.exec(path, args, envs).await?;
        // On success, execve() does not return, on error -1 is returned, and errno is
        // set to indicate the error.
        Ok(self.task.trap_context().result_value() as isize)
    }

    pub async fn sys_wait4(
        &self,
        pid: isize,
        status_addr: usize,
        options: usize,
        _rusage: usize,
    ) -> SyscallResult {
        trace!(
            "[sys_wait4] pid: {:?}, status_addr: {:?}, options: {:?}",
            pid,
            status_addr,
            options
        );
        let options = WaitOption::from_bits(options as i32).ok_or(Errno::EINVAL)?;
        let status: UserPtr<i32> = UserPtr::new(status_addr);

        // pid type
        // -1: all children, >0: specific pid, other: group unimplemented
        let pid_type = match pid {
            -1 => PidSel::Task(None),
            0 => PidSel::Group(None),
            pid if pid > 0 => PidSel::Task(Some(pid as usize)),
            pid => PidSel::Group(Some(pid as usize)),
        };

        // clone children info
        let children = self.task.pcb().children.clone();
        if children.is_empty() {
            return_errno!(Errno::ECHILD);
        }

        // work out target tasks
        let target_task = match pid_type {
            PidSel::Task(None) => {
                trace!("[sys_wait4] task {} wait for all children", self.task.tid());
                children.into_iter().find(|task| task.is_zombie())
            }
            PidSel::Task(Some(pid)) => {
                if let Some(task) = children.into_iter().find(|task| task.tid() == pid) {
                    task.is_zombie().then(|| task).or_else(|| None)
                } else {
                    return_errno!(Errno::ECHILD);
                }
            }
            PidSel::Group(_) => {
                error!("wait for process group is not implemented");
                return Err(Errno::EINVAL);
            }
        };

        // wait for target task
        let (target_tid, exit_code) = match target_task {
            Some(target_task) => {
                trace!("[sys_wait4] wait for task {}", target_task.tid());
                (target_task.tid(), target_task.exit_code())
            }
            None => {
                if options.contains(WaitOption::WNOHANG) {
                    return Ok(0);
                }
                let task = self.task;
                let (found_pid, exit_code) = loop {
                    task.set_wake_signal(!*task.sig_mask() | SigMask::SIGCHLD);
                    trace!("[sys_wait4] yield now, waiting for SIGCHLD");
                    // use polling instead of waker
                    suspend_now().await;
                    let sig_info = task.pending_sigs().pop_with_mask(SigMask::SIGCHLD);
                    if let Some(sig_info) = sig_info {
                        if let SigExtraInfo::Extend {
                            si_pid,
                            si_status,
                            si_stime: _,
                            si_utime: _,
                        } = sig_info.extra_info
                        {
                            match pid_type {
                                PidSel::Task(None) => break (si_pid, si_status),
                                PidSel::Task(target_pid) => {
                                    if si_pid as usize == target_pid.unwrap() {
                                        break (si_pid, si_status);
                                    }
                                }
                                PidSel::Group(_) => unimplemented!(),
                            }
                        }
                    } else {
                        return_errno!(Errno::EINTR);
                    }
                };
                (found_pid as usize, exit_code.unwrap())
            }
        };

        if !status.is_null() {
            trace!(
                "[sys_wait4]: write exit_code at status_addr = {:#x}",
                status.addr().0,
            );
            status.write_volatile((exit_code & 0xff) << 8);
            trace!("[sys_wait4]: write exit code {:#x}", exit_code);
        }
        self.task
            .pcb()
            .children
            .retain(|other| other.tid() != target_tid);
        TASK_MANAGER.remove(target_tid);
        Ok(target_tid as isize)
    }

    pub fn sys_getpid(&self) -> SyscallResult {
        Ok(self.task.tid() as isize)
    }

    pub fn sys_getppid(&self) -> SyscallResult {
        let parent_process = self.task.pcb().parent.clone();
        match parent_process {
            None => Err(Errno::ESRCH),
            Some(parent_process) => Ok(parent_process.upgrade().unwrap().tid() as isize),
        }
    }
}
