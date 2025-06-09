use alloc::sync::Arc;

use arch::{ArchTrapContext, TrapArgs, TrapContext};

use super::{utils::clear_current_syscall, SyscallResult};
use crate::{
    include::{result::Errno, syscall_id::SyscallID},
    syscall::utils::{current_syscall, update_current_syscall},
    task::Task,
};

/// system call tracer for a task
pub struct Syscall<'a> {
    pub task: &'a Arc<Task>,
}

#[rustfmt::skip]
impl<'a> Syscall<'a> {
    /// syscall implementation inner, with syscall lookup table
    async fn syscall_inner(&mut self, id: SyscallID, args: [usize; 6]) -> SyscallResult {
        use SyscallID::*;
        match id {
            // fs
            SYS_FCHMODAT =>         Self::empty_syscall("fchmodat", 0),
            SYS_UMASK =>            Self::empty_syscall("umask", 0x777),
            SYS_SYNC =>             Self::empty_syscall("sync", 0),
            SYS_FSYNC =>            Self::empty_syscall("fsync", 0),
            SYS_MSYNC =>            Self::empty_syscall("msync", 0),
            SYS_READ =>             self.sys_read(args[0], args[1], args[2]).await,
            SYS_READV =>            self.sys_readv(args[0], args[1], args[2]).await,
            SYS_PREAD64 =>          self.sys_pread64(args[0], args[1], args[2], args[3]).await,
            SYS_WRITE =>            self.sys_write(args[0], args[1], args[2]).await,
            SYS_WRITEV =>           self.sys_writev(args[0], args[1], args[2]).await,
            SYS_PWRITE64 =>         self.sys_pwrite64(args[0], args[1], args[2], args[3]).await,
            SYS_CLOSE =>            self.sys_close(args[0]),
            SYS_MKDIRAT =>          self.sys_mkdirat(args[0] as isize, args[1], args[2] as u32).await,
            SYS_OPENAT =>           self.sys_openat(args[0] as isize, args[1], args[2] as u32, args[3] as u32).await,
            SYS_CHDIR =>            self.sys_chdir(args[0]),
            SYS_GETCWD =>           self.sys_getcwd(args[0], args[1]).await,
            SYS_DUP =>              self.sys_dup(args[0]),
            SYS_DUP3 =>             self.sys_dup3(args[0], args[1]),
            SYS_PIPE2 =>            self.sys_pipe2(args[0], args[1]).await,
            SYS_FSTAT =>            self.sys_fstat(args[0], args[1]).await,
            SYS_GETDENTS64 =>       self.sys_getdents64(args[0], args[1], args[2]).await,
            SYS_MOUNT =>            self.sys_mount(args[0], args[1], args[2], args[3], args[4]).await,
            SYS_UMOUNT2 =>          self.sys_umount2(args[0], args[1]),
            SYS_LINKAT =>           self.sys_linkat(args[0] as isize,args[1],args[2] as isize,args[3],args[4]),
            SYS_UNLINKAT =>         self.sys_unlinkat(args[0] as isize, args[1], args[2]).await,
            SYS_PRLIMIT64 =>        self.sys_prlimit64(args[0], args[1] as u32, args[2], args[3]).await,
            SYS_FCNTL =>            self.sys_fcntl(args[0], args[1], args[2]),
            SYS_READLINKAT =>       self.sys_readlinkat(args[0] as isize, args[1], args[2], args[3]).await,
            SYS_IOCTL =>            self.sys_ioctl(args[0], args[1], args[2]).await,
            SYS_NEWFSTATAT =>       self.sys_newfstatat(args[0] as isize, args[1], args[2], args[3]).await,
            SYS_SENDFILE =>         self.sys_sendfile(args[0], args[1], args[2], args[3]).await,
            SYS_FACCESSAT =>        self.sys_faccessat(args[0] as isize, args[1], args[2], args[3]).await,
            SYS_UTIMENSAT =>        self.sys_utimensat(args[0] as isize, args[1], args[2], args[3]).await,
            SYS_LSEEK =>            self.sys_lseek(args[0], args[1] as isize, args[2]),
            SYS_RENAMEAT2 =>        self.sys_renameat2(args[0] as isize, args[1], args[2] as isize, args[3], args[4] as i32).await,
            SYS_COPY_FILE_RANGE =>  unimplemented!("shouldn't run SYS_COPY_FILE_RANGE"),
            SYS_FTRUNCATE64 =>      self.sys_ftruncate(args[0], args[1]).await,
            SYS_STATFS =>           self.sys_statfs(args[0], args[1]).await,
            SYS_SPLICE =>           self.sys_splice(args[0], args[1], args[2], args[3], args[4], args[5]).await,
            SYS_STATX =>            self.sys_statx(args[0] as isize, args[1], args[2] as u32, args[3] as u32, args[4]).await,

            // io
            SYS_PPOLL =>    self.sys_ppoll(args[0], args[1], args[2], args[3]).await,
            SYS_PSELECT =>  self.sys_pselect6(args[0], args[1], args[2], args[3], args[4], args[5]).await,

            // net
            SYS_SOCKET =>       self.sys_socket(args[0], args[1], args[2]),
            SYS_BIND =>         self.sys_bind(args[0], args[1], args[2]).await,
            SYS_LISTEN =>       self.sys_listen(args[0], args[1]).await,
            SYS_ACCEPT =>       self.sys_accept(args[0], args[1], args[2]).await,
            SYS_CONNECT =>      self.sys_connect(args[0], args[1], args[2]).await,
            SYS_SOCKETPAIR =>   self.sys_socketpair(args[0] as isize, args[1] as isize, args[2] as isize, args[3]).await,
            // SYS_ACCEPT4 =>      todo!(),
            SYS_SENDTO =>       self.sys_sendto(args[0], args[1], args[2], args[3] as u32, args[4], args[5]).await,
            SYS_RECVFROM =>     self.sys_recvfrom(args[0], args[1], args[2], args[3] as u32, args[4], args[5]).await,
            SYS_SHUTDOWN =>     self.sys_shutdown(args[0], args[1]).await,
            SYS_GETSOCKNAME =>  self.sys_getsockname(args[0], args[1], args[2]).await,
            SYS_GETPEERNAME =>  self.sys_getpeername(args[0], args[1], args[2]).await,
            SYS_SETSOCKOPT =>   self.sys_setsockopt(args[0], args[1], args[2], args[3], args[4]).await,
            SYS_GETSOCKOPT =>   self.sys_getsockopt(args[0], args[1], args[2], args[3], args[4]).await,

            // process
            SYS_GETUID =>               Self::empty_syscall("getuid", 0),
            SYS_GETEUID =>              Self::empty_syscall("geteuid", 0),
            SYS_GETGID =>               Self::empty_syscall("getgid", 0),
            SYS_GETEGID =>              Self::empty_syscall("getegid", 0),
            SYS_EXIT =>                 self.sys_exit(args[0] as i32),
            SYS_EXIT_GROUP =>           self.sys_exit_group(args[0] as i32),
            SYS_CLONE => {
                /*
                 * On x86-32, and several other common architectures (including
                 * score, ARM, ARM 64, PA-RISC, arc, Power PC, xtensa, and MIPS), the
                 * order of the last two arguments is reversed.
                 * And so on loongarch64.
                 * ref1: https://www.man7.org/linux/man-pages/man2/clone.2.html#VERSIONS
                 * ref2: https://inbox.vuxu.org/musl/1a5a097f.12d7.1794a6de3a8.Coremail.zhaixiaojuan%40loongson.cn/t/
                 * sys_clone(u64 flags, u64 ustack_base, u64 parent_tidptr, u64 child_tidptr, u64 tls)
                 */
                #[cfg(target_arch = "loongarch64")]
                let x = self.sys_clone(args[0], args[1], args[2], /* here */ args[4], args[3]).await;
                #[cfg(target_arch = "riscv64")]
                let x = self.sys_clone(args[0], args[1], args[2], args[3], args[4]).await;
                x
            }
            SYS_CLONE3 =>               self.sys_clone3(args[0], args[1]).await,
            SYS_EXECVE =>               self.sys_execve(args[0], args[1], args[2]).await,
            SYS_WAIT4 =>                self.sys_wait4(args[0] as isize, args[1], args[2]).await,
            SYS_GETTID =>               self.sys_gettid(),
            SYS_GETPID =>               self.sys_getpid(),
            SYS_GETPPID =>              self.sys_getppid(),
            SYS_SET_TID_ADDRESS =>      self.sys_set_tid_address(args[0]),
            SYS_GETPGID =>              self.sys_getpgid(args[0]),
            SYS_SETPGID =>              self.sys_setpgid(args[0], args[1]),
            SYS_GET_ROBUST_LIST =>      self.sys_get_robust_list(args[0], args[1], args[2]).await,
            SYS_SET_ROBUST_LIST =>      self.sys_set_robust_list(args[0], args[1]),
            SYS_FUTEX =>                self.sys_futex(args[0] as _, args[1] as _, args[2] as _, args[3] as _, args[4] as _, args[5] as _).await,
            SYS_SETSID =>               self.sys_setsid(),
            SYS_GETRUSAGE =>            self.sys_getrusage(args[0] as _, args[1]).await,
            
            // signal
            SYS_SIGTIMEDWAIT => Self::empty_syscall("sigtimedwait", 0),
            SYS_SIGACTION =>    self.sys_sigaction(args[0] as i32, args[1], args[2]).await,
            SYS_SIGRETURN =>    self.sys_sigreturn().await,
            SYS_KILL =>         self.sys_kill(args[0] as isize, args[1] as i32),
            SYS_TKILL =>        self.sys_tkill(args[0], args[1] as i32),
            SYS_SIGPROCMASK =>  self.sys_sigprocmask(args[0], args[1], args[2], args[3]).await,
            SYS_SIGSUSPEND =>   self.sys_sigsuspend(args[0]).await,
            SYS_TGKILL =>       self.sys_tgkill(args[0], args[1], args[2] as _),

            // mm
            SYS_MEMBARRIER =>   Self::empty_syscall("membarrier", 0), // fixme: should impl this in multicore
            SYS_MADVISE =>      Self::empty_syscall("madvise", 0),
            SYS_BRK =>          self.sys_brk(args[0]),
            SYS_MMAP =>         self.sys_mmap(args[0],args[1],args[2],args[3],args[4] as isize, args[5]),
            SYS_MUNMAP =>       self.sys_munmap(args[0], args[1]),
            SYS_MPROTECT =>     self.sys_mprotect(args[0], args[1], args[2]),
            SYS_SHMGET =>       self.sys_shmget(args[0], args[1], args[2]),
            SYS_SHMCTL =>       self.sys_shmctl(args[0], args[1], args[2] as *const u8),
            SYS_SHMAT =>        self.sys_shmat(args[0], args[1], args[2]),
            SYS_SHMDT =>        self.sys_shmdt(args[0]),
            
            // sched
            SYS_SCHED_YIELD =>          self.sys_yield().await,
            SYS_SCHED_GETAFFINITY =>    self.sys_sched_getaffinity(args[0], args[1], args[2]).await,
            SYS_SCHED_SETAFFINITY =>    self.sys_sched_setaffinity(args[0], args[1], args[2]).await,
            SYS_SCHEED_GETSCHEDULER =>  self.sys_sched_getscheduler(args[0]),
            SYS_SCHED_GETPARAM =>       self.sys_sched_getparam(args[0], args[1]).await,
            SYS_SCHED_SETSCHEDULER =>   self.sys_sched_setscheduler(args[0], args[1] as _, args[2]),

            // time
            SYS_TIMES =>            self.sys_times(args[0]).await,
            SYS_GETTIMEOFDAY =>     Self::sys_gettimeofday(args[0]).await,
            SYS_NANOSLEEP =>        self.sys_nanosleep(args[0], args[1]).await,
            SYS_CLOCK_GETTIME =>    self.sys_clock_gettime(args[0], args[1]).await,
            SYS_CLOCK_NANOSLEEP =>  self.sys_clock_nanosleep(args[0], args[1], args[2], args[3]).await,
            SYS_CLOCK_GETRES =>     self.sys_clock_getres(args[0], args[1]).await,
            SYS_SETITIMER =>        self.sys_setitimer(args[0] as _, args[1] as _, args[2] as _).await,
            SYS_GETITIMER =>        self.sys_getitimer(args[0] as _, args[1] as _).await,

            // system / others
            SYS_SYSINFO =>         Self::empty_syscall("info", 0),
            SYS_UNAME =>           Self::sys_uname(args[0]).await,
            SYS_SYSLOG =>          Self::sys_syslog(args[0] as u32, args[1], args[2]).await,
            SYS_SYSTEMSHUTDOWN =>  Self::sys_systemshutdown(),
            SYS_GETRANDOM =>       self.sys_getrandom(args[0], args[1], args[2]).await,
            SYS_GET_MEMPOLICY =>   Self::empty_syscall("get_mempolicy", 0),

            // unsupported
            _ => {
                println!(
                    "[kernel] unsupported syscall id: {:?}, tid: {}, args: {:x?}",
                    id, self.task.tid(), args,
                );
                Err(Errno::ENOSYS)
            }
        }
    }
}

impl<'a> Syscall<'a> {
    pub fn new(task: &'a Arc<Task>) -> Self {
        Self { task }
    }
    /// syscall implementation with debug info
    pub async fn syscall(&mut self, id: usize, args: [usize; 6]) -> SyscallResult {
        let id = SyscallID::from_repr(id as _).ok_or_else(|| {
            error!("invalid syscall id: {}", id);
            Errno::ENOSYS
        })?;
        update_current_syscall(id);
        if id.is_debug_on() {
            let cx = self.task.trap_context();
            use arch::TrapArgs::*;
            info!(
                "[syscall] id: {:?}, tid: {}, sp: {:#x}, pc: {:#x}, ra: {:#x}, args: {:X?}",
                id,
                self.task.tid(),
                cx[SP],
                cx[EPC],
                cx[RA],
                args
            );
        }
        let res = self.syscall_inner(id, args).await;
        if id.is_debug_on() {
            info!("[syscall(out)] id: {:?}, res: {:x?}", id, res);
        }
        // crate::utils::loghook::log_hook();
        // intermit(|| unsafe {
        //     println!(
        //         "[PageCache] holds frames: {}",
        //         FRAME_ALLOCS.load(core::sync::atomic::Ordering::SeqCst)
        //     )
        // });
        res
    }
    fn empty_syscall(name: &str, res: isize) -> SyscallResult {
        info!("[sys_{}] do nothing.", name);
        Ok(res)
    }
}

impl Task {
    pub async fn syscall(self: &Arc<Self>, cx: &mut TrapContext) -> isize {
        cx[TrapArgs::EPC] += 4;
        let res = get_syscall_result(
            Syscall::new(self)
                .syscall(cx.get_syscall_id(), cx.get_syscall_args())
                .await,
        );
        cx[TrapArgs::RES] = res as usize;
        clear_current_syscall();
        res
    }
}

pub fn get_syscall_result(res: SyscallResult) -> isize {
    match res {
        Ok(res) => res,
        Err(errno) => {
            warn!("syscall error: {:?} during {:?}", errno, current_syscall());
            let errno: isize = errno as isize;
            match errno > 0 {
                true => -errno,
                false => errno,
            }
        }
    }
}
