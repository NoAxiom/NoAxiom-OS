//! memory management system calls
use super::{Syscall, SyscallResult};
use crate::{
    fs::path::Path, mm::user_ptr::UserPtr, nix::clone_flags::CloneFlags, sched::task::spawn_utask,
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
        trace!(
            "[sys_fork] flags: {:x} stack: {:?} ptid: {:?} tls: {:?} ctid: {:?}",
            flags,
            stack,
            ptid,
            tls,
            ctid
        );
        let flags = CloneFlags::from_bits_truncate(flags);
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
}
