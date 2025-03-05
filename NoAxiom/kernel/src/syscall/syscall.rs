use alloc::sync::Arc;

use arch::{Arch, ArchInt, TrapContext};

use super::SyscallResult;
use crate::{constant::syscall::*, include::result::Errno, task::Task};

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
            assert!(!Arch::is_interrupt_enabled());
            Arch::enable_global_interrupt();
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
                    .await
            }
            SYS_CHDIR => self.sys_chdir(args[0]),
            SYS_GETCWD => self.sys_getcwd(args[0], args[1]).await,
            SYS_DUP => self.sys_dup(args[0]),
            SYS_DUP3 => self.sys_dup3(args[0], args[1]),
            SYS_PIPE2 => self.sys_pipe2(args[0], args[1]).await,
            SYS_FSTAT => self.sys_fstat(args[0], args[1]),

            // process
            SYS_EXIT => self.sys_exit(args[0]),
            SYS_CLONE => self.sys_fork(args[0], args[1], args[2], args[3], args[4]),
            SYS_EXECVE => self.sys_exec(args[0], args[1], args[2]).await,
            SYS_WAIT4 => self.sys_wait4(args[0], args[1], args[2], args[3]).await,
            SYS_GETPID => self.sys_getpid(),
            SYS_GETPPID => self.sys_getppid(),

            // mm
            SYS_BRK => self.sys_brk(args[0]),
            SYS_MMAP => self.sys_mmap(
                args[0],
                args[1],
                args[2],
                args[3],
                args[4] as isize,
                args[5],
            ),
            SYS_MUNMAP => self.sys_munmap(args[0], args[1]),

            // others
            SYS_TIMES => Self::sys_times(args[0]),
            SYS_SCHED_YIELD => Self::sys_yield().await,

            // unsupported: return -1
            _ => {
                error!("unsupported syscall id: {}, args: {:?}", id, args);
                let _ = self.sys_exit(Errno::ENOSYS as usize);
                Err(Errno::ENOSYS)
            }
        }
    }
}

pub async fn syscall(task: &Arc<Task>, cx: &TrapContext) -> isize {
    let res = Syscall::new(task)
        .syscall(cx.get_syscall_id(), cx.get_syscall_args())
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
