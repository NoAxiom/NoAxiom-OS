use super::{Syscall, SyscallResult};
use crate::{
    fs::path::Path,
    include::{
        process::{PidSel, WaitOption},
        result::Errno,
        sched::CloneFlags,
    },
    mm::user_ptr::UserPtr,
    sched::spawn::spawn_utask,
    task::{manager::TASK_MANAGER, wait::WaitChildFuture},
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
            let cwd = self.task.cwd().clone().from_cd(&"..");
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
        debug!(
            "[sys_wait4] pid: {:?}, status_addr: {:?}, options: {:?}",
            pid, status_addr, options
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
            WaitChildFuture::new(self.task.clone(), pid_type, wait_option)?.await?;
        if !status.is_null() {
            trace!(
                "[sys_wait4]: write exit_code at status_addr = {:#x}",
                status.addr().0,
            );
            status.write_volatile((exit_code & 0xff) << 8);
            trace!("[sys_wait4]: write exit code {:#x}", exit_code);
        }
        Ok(tid as isize)
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

/*

注意！如果事件发生没有严格的先后顺序，那么不能使用suspend让权！
我们来比较一下磁盘IO与等待子进程的情况：
磁盘IO拥有严格的顺序，一定是先发起IO请求，然后等待IO完成，所以可以使用suspend让权。
而等待子进程的情况下，子进程可能在 父进程寻找zombie进程 到 父进程让权 这段时间内检测父进程是否为suspend状态
由于父进程执行wait是无锁的，并不是一个原子的操作，因此假如是上面这种情况就会导致子进程误认为父进程未suspend
但父进程已经处于suspend的执行流当中了，这就导致父进程suspend之后无法再被唤醒！

*/
