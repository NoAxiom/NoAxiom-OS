use alloc::sync::Arc;

use arch::{Arch, ArchInt, ArchTrapContext, TrapContext};

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
            Arch::enable_interrupt();
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
            SYS_GETDENTS64 => self.sys_getdents64(args[0], args[1], args[2]).await,
            SYS_MOUNT => {
                self.sys_mount(args[0], args[1], args[2], args[3], args[4])
                    .await
            }
            SYS_UMOUNT2 => self.sys_umount2(args[0], args[1]),
            SYS_LINKAT => self.sys_linkat(args[0], args[1], args[2], args[3], args[4]),
            SYS_UNLINKAT => self.sys_unlinkat(args[0], args[1], args[2]).await,

            // net
            SYS_SOCKET => self.sys_socket(args[0], args[1], args[2]),
            SYS_BIND => self.sys_bind(args[0], args[1], args[2]),
            SYS_LISTEN => self.sys_listen(args[0], args[1]),
            SYS_ACCEPT => self.sys_accept(args[0], args[1], args[2]).await,
            SYS_CONNECT => self.sys_connect(args[0], args[1], args[2]).await,

            // process
            SYS_EXIT => self.sys_exit(args[0]),
            SYS_CLONE => self.sys_fork(args[0], args[1], args[2], args[3], args[4]),
            SYS_EXECVE => self.sys_exec(args[0], args[1], args[2]).await,
            SYS_WAIT4 => {
                self.sys_wait4(args[0] as isize, args[1], args[2], args[3])
                    .await
            }
            SYS_GETPID => self.sys_getpid(),
            SYS_GETPPID => self.sys_getppid(),

            // signal
            SYS_SIGACTION => self.sys_sigaction(args[0] as i32, args[1], args[2]),
            SYS_SIGRETURN => self.sys_sigreturn(),
            SYS_KILL => self.sys_kill(args[0] as isize, args[1] as i32),
            SYS_SIGPROCMASK => self.sys_sigprocmask(args[0], args[1], args[2], args[3]),
            SYS_SIGSUSPEND => self.sys_sigsuspend(args[0]).await,

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
            SYS_SCHED_YIELD => self.sys_yield().await,
            SYS_UNAME => Self::sys_uname(args[0]),
            SYS_GETTIMEOFDAY => Self::sys_gettimeofday(args[0]),
            SYS_NANOSLEEP => self.sys_nanosleep(args[0]).await,

            // unsupported: return -1
            _ => {
                error!("unsupported syscall id: {}, args: {:?}", id, args);
                let _ = self.sys_exit(Errno::ENOSYS as usize);
                Err(Errno::ENOSYS)
            }
        }
    }
}

impl Task {
    pub async fn syscall(self: &Arc<Self>, cx: &TrapContext) -> isize {
        let res = Syscall::new(self)
            .syscall(cx.get_syscall_id(), cx.get_syscall_args())
            .await;
        match res {
            Ok(res) => res,
            Err(errno) => {
                error!("syscall error: {:?}", errno);
                let errno = errno as isize;
                match errno > 0 {
                    true => -errno,
                    false => errno,
                }
            }
        }
    }
}
