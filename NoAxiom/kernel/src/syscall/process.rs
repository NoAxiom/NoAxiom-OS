//! memory management system calls
use super::{Syscall, SyscallResult};
use crate::{
    fs::path::Path,
    mm::user_ptr::UserPtr,
    nix::{
        clone_flags::CloneFlags,
        process::{PidSel, WaitOption},
        result::Errno,
    },
    return_errno,
    sched::task::spawn_utask,
    syscall::A0,
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
        debug!(
            "[sys_fork] flags: {:x} stack: {:?} ptid: {:?} tls: {:?} ctid: {:?}",
            flags, stack, ptid, tls, ctid
        );
        let flags = CloneFlags::from_bits(flags & !0xff).unwrap();
        let task = self.task.fork(flags);
        let trap_cx = task.trap_context_mut();
        if stack != 0 {
            trap_cx.set_sp(stack);
        }
        trace!("[sys_fork] new task context: {:?}", trap_cx);
        let tid = task.tid();
        spawn_utask(task);
        Ok(tid as isize)
    }

    pub async fn sys_exec(&mut self, path: usize, argv: usize, envp: usize) -> SyscallResult {
        let path = Path::new(UserPtr::new(path).get_cstr());
        info!("[sys_exec] path: {}", path.inner());
        let args = UserPtr::<u8>::new(argv).get_string_vec();
        let envs = UserPtr::<u8>::new(envp).get_string_vec();
        self.task.exec(path, args, envs).await;
        // On success, execve() does not return, on error -1 is returned, and errno is
        // set to indicate the error.
        Ok(self.task.trap_context().user_reg[A0] as isize)
    }

    pub async fn sys_wait4(
        &self,
        pid: usize,
        status_addr: usize,
        options: usize,
        _rusage: usize,
    ) -> SyscallResult {
        info!(
            "[sys_wait4] pid: {:?}, status_addr: {:?}, options: {:?}",
            pid, status_addr, options
        );
        let pid = pid as isize;
        let options = WaitOption::from_bits(options as i32).ok_or(Errno::EINVAL)?;
        let status: UserPtr<i32> = UserPtr::new(status_addr);

        // clone children info
        let children = self.task.pcb().children.clone();
        if children.is_empty() {
            return_errno!(Errno::ECHILD);
        }

        // pid type
        // -1: all children, >0: specific pid, other: group unimplemented
        let pid_type = match pid {
            -1 => PidSel::Task(None),
            0 => PidSel::Group(None),
            pid if pid > 0 => PidSel::Task(Some(pid as usize)),
            pid => PidSel::Group(Some(pid as usize)),
        };

        // work out target tasks
        let target_task = match pid_type {
            PidSel::Task(None) => {
                info!("[sys_wait4] task {} wait for all children", self.task.tid());
                children.into_iter().find(|task| task.is_zombie())
            }
            PidSel::Task(Some(pid)) => {
                if let Some(task) = children.iter().find(|task| task.tid() == pid).cloned() {
                    task.is_zombie().then(|| task).or_else(|| {
                        error!("[sys_wait4] task {} not found", pid);
                        None
                    })
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
        match target_task {
            Some(target_task) => {
                info!("[sys_wait4] wait for task {}", target_task.tid());
                if !status.is_null() {
                    info!(
                        "[sys_wait4]: write exit_code at status_addr = {:#x}",
                        status.addr(),
                    );
                    let exit_code = target_task.exit_code();
                    status.write_volatile((exit_code & 0xff) << 8);
                    info!("[sys_wait4]: write exit code {:#x}", exit_code);
                }
                let target_tid = target_task.tid();
                target_task
                    .pcb()
                    .children
                    .retain(|other| other.tid() != target_tid);
                TASK_MANAGER.remove(target_tid);
                Ok(target_tid as isize)
            }
            None => {
                error!("unimplemented");
                Err(Errno::ECHILD)
                // let task = self.task;
                // let (found_pid, exit_code) = loop {
                //     task.set_wake_up_signal(!*task.sig_mask() |
                // SigMask::SIGCHLD);     info!("suspend_now");
                //     suspend_now().await; // 这里利用了无栈协程的机制
                //     let siginfo =
                // task.pending_signals.lock().dequeue_except(SigMask::SIGCHLD);
                //     // 根据siginfo中传递的pid信息和status信息实现进程的退出和回收
                //     if let Some(siginfo) = siginfo {
                //         if let OtherInfo::Extend {
                //             si_pid,
                //             si_status,
                //             si_stime: _,
                //             si_utime: _,
                //         } = siginfo.otherinfo
                //         {
                //             match target {
                //                 PidSelection::Task(None) => break (si_pid,
                // si_status),
                // PidSelection::Task(target_pid) => {
                //                     if si_pid as usize == target_pid.unwrap()
                // {                         break (si_pid,
                // si_status);                     }
                //                 }
                //                 PidSelection::Group(_) => unimplemented!(),
                //                 PidSelection::Group(None) =>
                // unimplemented!(),             }
                //         }
                //     } else {
                //         return_errno!(Errno::EINTR);
                //     }
                // };
                // if exit_status_addr != 0 {
                //     task.check_lazy(exit_status_addr.into());
                //     info!(
                //         "[sys_waitpid]: write pid to exit_status_ptr {:#x}
                // before",         exit_status_addr
                //     );
                //     let exit_status_ptr = exit_status_addr as *mut i32;
                //     unsafe {
                //         exit_status_ptr.write_volatile((exit_code.unwrap() &
                // 0xff) << 8);         info!(
                //             "[sys_waitpid]: write pid to exit_code_ptr after,
                // exit code {:#x}",
                // (*exit_status_ptr & 0xff00) >> 8         );
                //     };
                // }
                // // 将进程从子进程数组和全局任务管理器中删除，
                // 至此进程彻底被回收 //
                // rust的资源回收不需要一个个手动回收资源，
                // 只需要彻底回收资源所有者， // 其资源通过rust机制自然回收
                // task.pcb()
                //     .children
                //     .retain(|x| x.get_taskid() != found_pid as usize);
                // TASK_MANAGER.remove(found_pid as usize);
                // Ok(found_pid as isize)
            }
        }
    }
}
