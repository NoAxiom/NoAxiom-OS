use alloc::sync::Arc;

use arch::{ArchTrapContext, TrapContext};

use super::SyscallResult;
use crate::{
    include::{result::Errno, syscall::SyscallID},
    task::Task,
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
        #[cfg(feature = "interruptable_async")]
        {
            // fixme: turn on the interrupt. When call trap_handler, cpu would turn off
            // the interrupt until cpu ertn. But if we switch to another task, the whole
            // life time is in the interrupt off state until previous task ertn.
            use arch::ArchInt;
            assert!(!arch::Arch::is_interrupt_enabled());
            arch::Arch::enable_interrupt();
        }
        let id = SyscallID::from_repr(id as usize).ok_or_else(|| {
            error!("invalid syscall id: {}", id);
            Errno::ENOSYS
        })?;
        trace!("[syscall] id: {:?}, args: {:X?}", id, args);
        use SyscallID::*;
        #[rustfmt::skip]
        let res = match id {
            // fs
            SYS_FCHMODAT =>         Self::empty_syscall("fchmodat", 0),
            SYS_UMASK =>            Self::empty_syscall("umask", 0x777),
            SYS_SYNC =>             Self::empty_syscall("sync", 0),
            SYS_FSYNC =>            Self::empty_syscall("fsync", 0),
            SYS_MSYNC =>            Self::empty_syscall("msync", 0),
            SYS_READ =>             self.sys_read(args[0], args[1], args[2]).await,
            SYS_READV =>            self.sys_readv(args[0], args[1], args[2]).await,
            SYS_WRITE =>            self.sys_write(args[0], args[1], args[2]).await,
            SYS_WRITEV =>           self.sys_writev(args[0], args[1], args[2]).await,
            SYS_CLOSE =>            self.sys_close(args[0]),
            SYS_MKDIRAT =>          self.sys_mkdirat(args[0] as isize, args[1], args[2] as u32).await,
            SYS_OPENAT =>           self.sys_openat(args[0] as isize, args[1], args[2] as u32, args[3] as u32).await,
            SYS_CHDIR =>            self.sys_chdir(args[0]),
            SYS_GETCWD =>           self.sys_getcwd(args[0], args[1]).await,
            SYS_DUP =>              self.sys_dup(args[0]),
            SYS_DUP3 =>             self.sys_dup3(args[0], args[1]),
            SYS_PIPE2 =>            self.sys_pipe2(args[0], args[1]).await,
            SYS_FSTAT =>            self.sys_fstat(args[0], args[1]),
            SYS_GETDENTS64 =>       self.sys_getdents64(args[0], args[1], args[2]).await,
            SYS_MOUNT =>            self.sys_mount(args[0], args[1], args[2], args[3], args[4]).await,
            SYS_UMOUNT2 =>          self.sys_umount2(args[0], args[1]),
            SYS_LINKAT =>           self.sys_linkat(args[0] as isize,args[1],args[2] as isize,args[3],args[4]),
            SYS_UNLINKAT =>         self.sys_unlinkat(args[0] as isize, args[1], args[2]).await,
            SYS_PRLIMIT64 =>        self.sys_prlimit64(args[0], args[1] as u32, args[2], args[3]),
            SYS_FCNTL =>            self.sys_fcntl(args[0], args[1], args[2]),
            SYS_READLINKAT =>       self.sys_readlinkat(args[0] as isize, args[1], args[2], args[3]).await,
            SYS_IOCTL =>            self.sys_ioctl(args[0], args[1], args[2]),
            SYS_NEWFSTATAT =>       self.sys_newfstatat(args[0] as isize, args[1], args[2], args[3]).await,
            SYS_SENDFILE =>         self.sys_sendfile(args[0], args[1], args[2], args[3]).await,
            SYS_FACCESSAT =>        self.sys_faccessat(args[0] as isize, args[1], args[2], args[3]),
            SYS_UTIMENSAT =>        self.sys_utimensat(args[0] as isize, args[1], args[2], args[3]),
            // SYS_LSEEK =>            todo!(),
            // SYS_RENAMEAT2 =>        todo!(),
            // SYS_COPY_FILE_RANGE =>  todo!(),
            // SYS_FTRUNCATE64 =>      todo!(),
            // SYS_PREAD64 =>          todo!(),
            // SYS_PSELECT =>          todo!(),
            // SYS_STATFS =>           todo!(),
            // SYS_PWRITE64 =>         todo!(),
            // SYS_SPLICE =>           todo!(),

            // io
            SYS_PPOLL => self.sys_ppoll(args[0], args[1], args[2], args[3]).await,

            // net
            SYS_SOCKET =>       self.sys_socket(args[0], args[1], args[2]),
            SYS_BIND =>         self.sys_bind(args[0], args[1], args[2]),
            SYS_LISTEN =>       self.sys_listen(args[0], args[1]),
            SYS_ACCEPT =>       self.sys_accept(args[0], args[1], args[2]).await,
            SYS_CONNECT =>      self.sys_connect(args[0], args[1], args[2]).await,
            // SYS_SOCKETPAIR =>   todo!(),
            // SYS_ACCEPT4 =>      todo!(),
            // SYS_SENDTO =>       todo!(),
            // SYS_RECVFROM =>     todo!(),
            // SYS_SHUTDOWN =>     todo!(),
            // SYS_GETSOCKNAME =>  todo!(),
            // SYS_GETPEERNAME =>  todo!(),
            // SYS_SETSOCKOPT =>   todo!(),
            // SYS_GETSOCKOPT =>   todo!(),

            // process
            SYS_GETUID =>               Self::empty_syscall("getuid", 0),
            SYS_GETEUID =>              Self::empty_syscall("geteuid", 0),
            SYS_GETGID =>               Self::empty_syscall("getgid", 0),
            SYS_GETEGID =>              Self::empty_syscall("getegid", 0),
            SYS_EXIT =>                 self.sys_exit(args[0] as i32),
            SYS_EXIT_GROUP =>           self.sys_exit_group(args[0] as i32),
            SYS_CLONE =>                self.sys_fork(args[0], args[1], args[2], args[3], args[4]),
            SYS_EXECVE =>               self.sys_execve(args[0], args[1], args[2]).await,
            SYS_WAIT4 =>                self.sys_wait4(args[0] as isize, args[1], args[2], args[3]).await,
            SYS_GETTID =>               self.sys_gettid(),
            SYS_GETPID =>               self.sys_getpid(),
            SYS_GETPPID =>              self.sys_getppid(),
            SYS_SET_TID_ADDRESS =>      self.sys_set_tid_address(args[0]),
            SYS_GETPGID =>              self.sys_getpgid(args[0]),
            SYS_SETPGID =>              self.sys_setpgid(args[0], args[1]),
            // SYS_SCHED_GETAFFINITY =>    todo!(),
            // SYS_SCHED_SETAFFINITY =>    todo!(),
            // SYS_SCHEED_GETSCHEDULER =>  todo!(),
            // SYS_SCHED_GETPARAM =>       todo!(),
            // SYS_SCHED_SETSCHEDULER =>   todo!(),
            // SYS_TKILL =>                todo!(),
            // SYS_GETRUSAGE =>            todo!(),
            // SYS_SETSID =>               todo!(),
            // SYS_SYSTEMSHUTDOWN =>       todo!(),
            
            // futex
            SYS_GET_ROBUST_LIST => self.sys_get_robust_list(args[0], args[1], args[2]),
            SYS_SET_ROBUST_LIST => self.sys_set_robust_list(args[0], args[1]),

            // signal
            SYS_SIGTIMEDWAIT => Self::empty_syscall("sigtimedwait", 0),
            SYS_SIGACTION =>    self.sys_sigaction(args[0] as i32, args[1], args[2]),
            SYS_SIGRETURN =>    self.sys_sigreturn(),
            SYS_KILL =>         self.sys_kill(args[0] as isize, args[1] as i32),
            SYS_SIGPROCMASK =>  self.sys_sigprocmask(args[0], args[1], args[2], args[3]),
            SYS_SIGSUSPEND =>   self.sys_sigsuspend(args[0]).await,

            // mm
            SYS_MEMBARRIER =>   Self::empty_syscall("membarrier", 0),
            SYS_MADVISE =>      Self::empty_syscall("madvise", 0),
            SYS_BRK =>          self.sys_brk(args[0]),
            SYS_MMAP =>         self.sys_mmap(args[0],args[1],args[2],args[3],args[4] as isize, args[5]),
            SYS_MUNMAP =>       self.sys_munmap(args[0], args[1]),
            SYS_MPROTECT =>     self.sys_mprotect(args[0], args[1], args[2]),
            SYS_SHMGET =>       self.sys_shmget(args[0], args[1], args[2]),
            SYS_SHMCTL =>       self.sys_shmctl(args[0], args[1], args[2] as *const u8),
            SYS_SHMAT =>        self.sys_shmat(args[0], args[1], args[2]),
            SYS_SHMDT =>        self.sys_shmdt(args[0]),
            
            // others
            SYS_TIMES =>            self.sys_times(args[0]),
            SYS_SCHED_YIELD =>      self.sys_yield().await,
            SYS_GETTIMEOFDAY =>     Self::sys_gettimeofday(args[0]),
            SYS_NANOSLEEP =>        self.sys_nanosleep(args[0]).await,
            SYS_GETRANDOM =>        self.sys_getrandom(args[0], args[1], args[2]).await,
            // SYS_SETITIMER =>        todo!(),
            // SYS_CLOCK_GETTIME =>    todo!(),
            // SYS_CLOCK_GETRES =>     todo!(),
            // SYS_CLOCK_NANOSLEEP =>  todo!(),

            // system
            SYS_SYSINFO =>  Self::empty_syscall("info", 0),
            SYS_UNAME =>    Self::sys_uname(args[0]),
            SYS_SYSLOG =>   Self::sys_syslog(args[0], args[1], args[2]).await,

            // futex
            SYS_FUTEX => todo!(),

            // unsupported
            _ => {
                error!("unsupported syscall id: {:?}, args: {:#x?}", id, args);
                // let _ = self.sys_exit(Errno::ENOSYS as usize);
                Err(Errno::ENOSYS)
            }
        };
        trace!("[syscall(out)] syscall id: {:?}, res: {:?}", id, res);
        res
    }

    fn empty_syscall(name: &str, res: isize) -> SyscallResult {
        info!("[sys_{}] do nothing.", name);
        Ok(res)
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
