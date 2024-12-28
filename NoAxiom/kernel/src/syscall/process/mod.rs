//! memory management system calls
use alloc::vec::Vec;

use super::{Syscall, SyscallResult};
use crate::{
    fs::path::Path, nix::clone_flags::CloneFlags, sched::task::spawn_utask,
    utils::get_string_from_ptr,
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
            flags, stack, ptid, tls, ctid
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
        trace!("sys_exec");
        let path = Path::new(get_string_from_ptr(path as *const u8));

        let argv = argv as *const *const u8;
        let argv_vec = Vec::new();

        let envp = envp as *const *const u8;
        let envp_vec = Vec::new();

        self.task.exec(path, argv_vec, envp_vec).await;
        Ok(0)
    }
}
