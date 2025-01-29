//! memory management system calls
use bitflags::bitflags;

use super::{Syscall, SyscallResult};
use crate::{
    fs::path::Path,
    mm::user_ptr::UserPtr,
    nix::{clone_flags::CloneFlags, result::Errno},
    return_errno,
    sched::task::spawn_utask,
    syscall::A0,
};

impl Syscall<'_> {
    /// exit current task by marking it as zombie
    pub fn sys_exit(&mut self) -> SyscallResult {
        self.task.exit();
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

    pub async fn sys_wait4(&self, pid: usize, status_ptr: usize, options: usize) -> SyscallResult {
        let pid = pid as isize;
        let options = options as i32;
        bitflags! {
            struct WaitOption: i32 {
                const WNOHANG = 1 << 0;
                const WUNTRACED = 1 << 1;
                const WCONTINUED = 1 << 3;
            }
        }
        #[derive(Debug, Clone, Copy)]
        pub enum PidSelection {
            Group(Option<usize>),
            Task(Option<usize>),
        }
        //根据不同的pid值我们分离出不同的解决方式
        impl From<isize> for PidSelection {
            fn from(value: isize) -> Self {
                match value {
                    -1 => PidSelection::Task(None),
                    0 => PidSelection::Group(None),
                    x if x > 0 => PidSelection::Task(Some(x as usize)),
                    x => PidSelection::Group(Some(x as usize)),
                }
            }
        }
        info!("[sys_wait4]: enter, pid {}, options {:#x}", pid, options);
        let task = self.task;
        let options = WaitOption::from_bits(options).ok_or(Errno::EINVAL)?;
        let target: PidSelection = pid.into();
        let children = task.pcb().children.clone();
        if children.is_empty() {
            drop(children);
            return_errno!(Errno::ECHILD);
        }
        // 先检查一遍进程的子进程队列此时是否有符合条件的可回收的子进程，有的话直接执行
        let target_task = match target {
            PidSelection::Task(None) => children.into_iter().find(|task| task.is_zombie()),
            PidSelection::Group(None) => unimplemented!(),
            PidSelection::Task(pid) => {
                if let Some(task) = children
                    .iter()
                    .find(|task| task.tid() == pid.unwrap())
                    .cloned()
                {
                    if task.is_zombie() {
                        Some(task)
                    } else {
                        None
                    }
                } else {
                    return_errno!(Errno::ECHILD);
                }
            }
            PidSelection::Group(x) => unimplemented!(),
        };
        Ok(0)
        // todo!()
        // if let Some(res_task) = target_task {
        //     info!("sys_wait4 find a target task");
        //     if status_ptr != 0 {
        //         task.check_lazy(status_ptr.into());
        //         info!(
        //             "[sys_wait4]: write pid to exit_status_ptr {:#x} before",
        //             status_ptr
        //         );
        //         let exit_status_ptr = status_ptr as *mut i32;
        //         unsafe {
        //             exit_status_ptr.write_volatile((res_task.exitcode() &
        // 0xff) << 8);             info!(
        //                 "[sys_wait4]: write pid to exit_code_ptr after, exit
        // code {:#x}",                 (*exit_status_ptr & 0xff00) >> 8
        //             );
        //         };
        //     }
        //     let taskid = res_task.get_taskid();
        //     // 将进程从子进程数组和全局任务管理器中删除，至此进程彻底被回收
        //     // rust的资源回收不需要一个个手动回收资源，
        // 只需要彻底回收资源所有者，     // 其资源通过rust机制自然回收
        //     task.pcb().children.retain(|x| x.get_taskid() != taskid);
        //     TASK_MANAGER.remove(taskid);
        //     drop(res_task);
        //     return Ok(taskid as isize);
        // } else if options.contains(WaitOption::WNOHANG) {
        //     return Ok(0);
        // } else {
        //     // 先检查一遍进程的子进程队列此时是否有符合条件的可回收的子进程，
        //     // 没有的话此时任务会直接阻塞等待 这里利用了无栈协程的机制
        //     // 等待的事件就是SIGCHLD信号，直到进程向父进程发送这个信号后，
        //     // 父进程才从此处唤醒继续执行
        //     let (found_pid, exit_code) = loop {
        //         task.set_wake_up_signal(!*task.sig_mask() |
        // SigMask::SIGCHLD);         info!("suspend_now");
        //         suspend_now().await; // 这里利用了无栈协程的机制
        //         let siginfo =
        // task.pending_signals.lock().dequeue_except(SigMask::SIGCHLD);
        //         // 根据siginfo中传递的pid信息和status信息实现进程的退出和回收
        //         if let Some(siginfo) = siginfo {
        //             if let OtherInfo::Extend {
        //                 si_pid,
        //                 si_status,
        //                 si_stime: _,
        //                 si_utime: _,
        //             } = siginfo.otherinfo
        //             {
        //                 match target {
        //                     PidSelection::Task(None) => break (si_pid,
        // si_status),
        // PidSelection::Task(target_pid) => {
        // if si_pid as usize == target_pid.unwrap() {
        // break (si_pid, si_status);                         }
        //                     }
        //                     PidSelection::Group(_) => unimplemented!(),
        //                     PidSelection::Group(None) => unimplemented!(),
        //                 }
        //             }
        //         } else {
        //             return_errno!(Errno::EINTR);
        //         }
        //     };
        //     if status_ptr != 0 {
        //         task.check_lazy(status_ptr.into());
        //         info!(
        //             "[sys_waitpid]: write pid to exit_status_ptr {:#x}
        // before",             status_ptr
        //         );
        //         let exit_status_ptr = status_ptr as *mut i32;
        //         unsafe {
        //             exit_status_ptr.write_volatile((exit_code.unwrap() &
        // 0xff) << 8);             info!(
        //                 "[sys_waitpid]: write pid to exit_code_ptr after,
        // exit code {:#x}",                 (*exit_status_ptr & 0xff00)
        // >> 8             );
        //         };
        //     }
        //     // 将进程从子进程数组和全局任务管理器中删除，至此进程彻底被回收
        //     // rust的资源回收不需要一个个手动回收资源，
        // 只需要彻底回收资源所有者，     // 其资源通过rust机制自然回收
        //     task.pcb()
        //         .children
        //         .retain(|x| x.get_taskid() != found_pid as usize);
        //     TASK_MANAGER.remove(found_pid as usize);
        //     return Ok(found_pid as isize);
        // }
        // Ok(ret)
    }
}
