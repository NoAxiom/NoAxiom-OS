//! memory management system calls
use crate::syscall::nix::clone_flags::CloneFlags;

use super::Syscall;

impl Syscall<'_> {
    /// exit current task by marking it as zombie
    pub fn sys_exit(&mut self) -> isize {
        self.task.exit();
        0
    }

    pub fn sys_fork(
        &self,
        flags: usize, // 创建的标志，如SIGCHLD
        stack: usize, // 指定新进程的栈，可为0
        ptid: usize,  // 父线程ID
        tls: usize,   // TLS线程本地存储描述符
        ctid: usize,  // 子线程ID
    ) -> isize {
        trace!(
            "[sys_fork] flags: {:x} stack: {:?} ptid: {:?} tls: {:?} ctid: {:?}",
            flags,
            stack,
            ptid,
            tls,
            ctid
        );
        let flag = CloneFlags::from_bits_truncate(flags);
        if flag.contains(CloneFlags::THREAD) {
            self.task.fork_thread();
        } else {
            self.task.fork_process();
        }
        0
    }

    pub async fn sys_exec(&mut self) -> isize {
        trace!("sys_exec");
        todo!();
    }
}
