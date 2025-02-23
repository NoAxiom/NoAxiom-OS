use alloc::sync::Arc;

use super::SyscallResult;
use crate::{
    constant::{register::*, syscall::*},
    include::result::Errno,
    task::Task,
    trap::TrapContext,
};

/// system call tracer for a task
pub struct Syscall<'a> {
    pub task: &'a Arc<Task>,
}

impl<'a> Syscall<'a> {
    pub fn new(task: &'a Arc<Task>) -> Self {
        Self { task }
    }
    pub async fn syscall(&mut self, id: usize, args: [usize; 6]) -> SyscallResult {
        trace!("syscall id: {}, args: {:?}", id, args);
        #[cfg(feature = "async_fs")]
        {
            // fixme: turn on the interrupt. When call trap_handler, cpu would turn off
            // the interrupt until cpu ertn. But if we switch to another task, the whole
            // life time is in the interrupt off state until previous task ertn.
            assert!(!arch::interrupt::is_interrupt_enabled());
            arch::interrupt::enable_global_interrupt();
        }
        match id {
            // fs
            SYS_READ => self.sys_read(args[0], args[1], args[2]).await,
            SYS_WRITE => self.sys_write(args[0], args[1], args[2]).await,
            SYS_CLOSE => self.sys_close(args[0]),
            SYS_MKDIRAT => {
                self.sys_mkdirat(args[0] as isize, args[1], args[2] as u32)
                    .await
            }
            SYS_OPENAT => {
                self.sys_openat(args[0] as isize, args[1], args[2] as u32, args[3] as u32)
            }
            SYS_CHDIR => self.sys_chdir(args[0]),
            SYS_GETCWD => self.sys_getcwd(args[0] as *mut u8, args[1]),
            SYS_DUP => self.sys_dup(args[0]),
            SYS_DUP3 => self.sys_dup3(args[0], args[1]),
            SYS_PIPE2 => self.sys_pipe2(args[0] as *mut i32, args[1]),

            // process
            SYS_EXIT => self.sys_exit(args[0]),
            SYS_CLONE => self.sys_fork(args[0], args[1], args[2], args[3], args[4]),
            SYS_EXECVE => self.sys_exec(args[0], args[1], args[2]).await,
            SYS_WAIT4 => self.sys_wait4(args[0], args[1], args[2], args[3]).await,
            SYS_GETPID => self.sys_getpid(),

            // mm
            SYS_BRK => todo!(),
            SYS_MMAP => todo!(),
            SYS_MUNMAP => todo!(),

            // others
            SYS_TIMES => Self::sys_times(args[0]),
            SYS_SCHED_YIELD => Self::sys_yield().await,

            // unsupported: return -1
            _ => {
                error!("unsupported syscall id: {}, args: {:?}", id, args);
                Err(Errno::ENOSYS)
            }
        }
    }
}

pub async fn syscall(task: &Arc<Task>, cx: &TrapContext) -> isize {
    let res = Syscall::new(task)
        .syscall(
            cx.user_reg[A7],
            [
                cx.user_reg[A0],
                cx.user_reg[A1],
                cx.user_reg[A2],
                cx.user_reg[A3],
                cx.user_reg[A4],
                cx.user_reg[A5],
            ],
        )
        .await;
    match res {
        Ok(res) => res,
        Err(errno) => {
            error!("syscall error: {:?}", errno);
            let errno = errno as isize;
            if errno > 0 {
                -errno
            } else {
                errno
            }
        }
    }
}
