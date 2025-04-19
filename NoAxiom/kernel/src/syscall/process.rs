use alloc::{string::String, vec::Vec};

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
    task::{
        exit::ExitCode,
        manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
        wait::WaitChildFuture,
    },
};

impl Syscall<'_> {
    /// exit current task by marking it as zombie
    pub fn sys_exit(&mut self, exit_code: i32) -> SyscallResult {
        self.task.terminate(ExitCode::new(exit_code));
        Ok(0)
    }

    /// exit group
    pub fn sys_exit_group(&mut self, exit_code: i32) -> SyscallResult {
        // terminate_all_tasks();
        let task = self.task;
        let tasks = task.thread_group_map(|tgroup| {
            let mut tasks = Vec::new();
            for (_, task) in tgroup.0.iter_mut() {
                let task = task.upgrade().unwrap();
                tasks.push(task);
            }
            tasks
        });
        for t in tasks {
            t.terminate(ExitCode::new(exit_code));
        }
        task.terminate(ExitCode::new(exit_code));
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
        let flags = CloneFlags::from_bits(flags & !0xff).unwrap();
        let new_task = self.task.fork(flags);
        let new_tid = new_task.tid();
        let cx = new_task.trap_context_mut();
        debug!(
            "[sys_fork] flags: {:?} stack: {:?} ptid: {:?} tls: {:?} ctid: {:?}",
            flags, stack, ptid, tls, ctid
        );
        use TrapArgs::*;
        if stack != 0 {
            cx[SP] = stack;
        }
        if flags.contains(CloneFlags::SETTLS) {
            cx[TLS] = tls;
        }
        if flags.contains(CloneFlags::PARENT_SETTID) {
            let ptid = UserPtr::<usize>::new(ptid);
            ptid.write(new_tid);
        }
        if flags.contains(CloneFlags::CHILD_SETTID) {
            let ctid = UserPtr::<usize>::new(ctid);
            ctid.write(new_tid);
        }
        if flags.contains(CloneFlags::CHILD_CLEARTID) {
            new_task.set_clear_tid_address(ctid);
        }
        cx[RES] = 0;
        trace!("[sys_fork] new task context: {:?}", cx);
        spawn_utask(new_task);
        debug!("[sys_fork] done");
        Ok(new_tid as isize)
    }

    pub async fn sys_execve(&mut self, path: usize, argv: usize, envp: usize) -> SyscallResult {
        let mut path = UserPtr::new(path).get_cstr();
        let mut args = Vec::new();
        let mut envs = Vec::new();

        // args and envs init
        if path.contains(".sh") {
            path = String::from("busybox");
            args.push(String::from("busybox"));
            args.push(String::from("sh"));
        } else if path.ends_with("ls") || path.ends_with("sleep") {
            path = String::from("/glibc/busybox");
            args.push(String::from("busybox"));
        }
        envs.push(String::from("PATH=/glibc"));
        envs.push(String::from("LD_LIBRARY_PATH=/glibc"));

        let file_path = if !path.starts_with('/') {
            let cwd = self.task.cwd();
            debug!("[sys_exec] cwd: {:?}", cwd);
            cwd.from_cd(&path)?
        } else {
            Path::try_from(path)?
        };
        // append args and envs from user provided
        args.append(&mut UserPtr::<UserPtr<u8>>::new(argv).get_string_vec());
        envs.append(&mut UserPtr::<UserPtr<u8>>::new(envp).get_string_vec());

        info!(
            "[sys_exec] path: {:?} argv: {:#x}, envp: {:#x}, arg: {:?}, env: {:?}",
            file_path, argv, envp, args, envs,
        );
        self.task.execve(file_path, args, envs).await?;

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
            status.write(ExitCode::new(exit_code).inner());
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
            PROCESS_GROUP_MANAGER.insert_new_group(&target_task);
        } else {
            match PROCESS_GROUP_MANAGER.get_group(pgid) {
                Some(_) => PROCESS_GROUP_MANAGER.insert_process(pgid, &target_task),
                None => PROCESS_GROUP_MANAGER.insert_new_group(&target_task),
            }
        }
        Ok(0)
    }
}
