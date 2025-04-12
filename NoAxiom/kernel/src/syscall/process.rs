use arch::TrapArgs;

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
    task::wait::WaitChildFuture,
};

impl Syscall<'_> {
    /// exit current task by marking it as zombie
    pub fn sys_exit(&mut self, exit_code: usize) -> SyscallResult {
        let exit_code = exit_code as i32;
        self.task.terminate(exit_code);
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
            "[sys_fork] flags: {:?} stack: {:?} ptid: {:?} tls: {:?} ctid: {:?}",
            flags, stack, ptid, tls, ctid
        );
        let flags = CloneFlags::from_bits(flags & !0xff).unwrap();
        let task = self.task.fork(flags);
        let trap_cx = task.trap_context_mut();
        let tid = task.tid();
        use TrapArgs::*;
        if stack != 0 {
            trap_cx[SP] = stack;
        }
        // TODO: PARENT_SETTID CHILD_SETTID CHILD_CLEARTID
        if flags.contains(CloneFlags::SETTLS) {
            trap_cx[TLS] = tls;
        }
        trap_cx[RES] = 0;
        trace!("[sys_fork] new task context: {:?}", trap_cx);
        spawn_utask(task);
        debug!("[sys_fork] done");
        Ok(tid as isize)
    }

    pub async fn sys_execve(&mut self, path: usize, argv: usize, envp: usize) -> SyscallResult {
        let path = UserPtr::new(path).get_cstr();
        let path = if !path.starts_with('/') {
            let cwd = self.task.cwd().clone().from_cd(&"..");
            trace!("[sys_exec] cwd: {:?}", cwd);
            cwd.from_cd(&path)
        } else {
            Path::from(path)
        };
        let args = UserPtr::<UserPtr<u8>>::new(argv).get_string_vec();
        let envs = UserPtr::<UserPtr<u8>>::new(envp).get_string_vec();
        info!(
            "[sys_exec] path: {:?} argv: {:#x}, envp: {:#x}, arg: {:?}, env: {:?}",
            path, argv, envp, args, envs,
        );
        self.task.execve(path, args, envs).await?;
        // On success, execve() does not return, on error -1 is returned, and errno is
        // set to indicate the error.
        Ok(self.task.trap_context()[TrapArgs::RES] as isize)
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
            status.write((exit_code & 0xff) << 8);
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

    pub fn sys_set_tid_address(&self, tidptr: usize) -> SyscallResult {
        let task = self.task;
        task.set_clear_tid_address(tidptr);
        Ok(task.tid() as isize)
    }
}
