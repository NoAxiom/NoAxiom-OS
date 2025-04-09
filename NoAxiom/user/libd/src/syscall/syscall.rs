#![allow(unused)]
use core::arch::asm;

use crate::{syscall, syscall_id};

syscall_id!(SYS_GETCWD, 17);
syscall_id!(SYS_PIPE2, 59);
syscall_id!(SYS_DUP, 23);
syscall_id!(SYS_DUP3, 24);
syscall_id!(SYS_CHDIR, 49);
syscall_id!(SYS_OPENAT, 56);
syscall_id!(SYS_CLOSE, 57);
syscall_id!(SYS_GETDENTS64, 61);
syscall_id!(SYS_READ, 63);
syscall_id!(SYS_WRITE, 64);
syscall_id!(SYS_LINKAT, 37);
syscall_id!(SYS_UNLINKAT, 35);
syscall_id!(SYS_MKDIRAT, 34);
syscall_id!(SYS_UMOUNT2, 39);
syscall_id!(SYS_MOUNT, 40);
syscall_id!(SYS_FSTAT, 80);
syscall_id!(SYS_CLONE, 220);
syscall_id!(SYS_EXECVE, 221);
syscall_id!(SYS_WAIT4, 260);
syscall_id!(SYS_EXIT, 93);
syscall_id!(SYS_GETPPID, 173);
syscall_id!(SYS_GETPID, 172);
syscall_id!(SYS_BRK, 214);
syscall_id!(SYS_MUNMAP, 215);
syscall_id!(SYS_MMAP, 222);
syscall_id!(SYS_TIMES, 153);
syscall_id!(SYS_UNAME, 160);
syscall_id!(SYS_SCHED_YIELD, 124);
syscall_id!(SYS_GETTIMEOFDAY, 169);
syscall_id!(SYS_NANOSLEEP, 101);
syscall_id!(SYSCALL_SYSTEMSHUTDOWN, 2003);
syscall_id!(SYSCALL_FRAMEBUFFER, 1002);
syscall_id!(SYSCALL_FRAMEBUFFER_FLUSH, 1003);
syscall_id!(SYSCALL_EVENT_GET, 1004);
syscall_id!(SYSCALL_LISTEN, 1005);
syscall_id!(SYSCALL_CONNNET, 1006);


// 传入系统调用号和相应系统调用及其参数
syscall!(sys_fork, SYS_CLONE);

syscall!(
    sys_clone,
    SYS_CLONE,
    fn(*const u8) -> isize,
    *const u8,
    i32,
    *const u8
);

syscall!(
    sys_execve,
    SYS_EXECVE,
    *const u8,
    *const usize,
    *const usize
);

syscall!(sys_sched_yield, SYS_SCHED_YIELD);

syscall!(sys_waitpid, SYS_WAIT4, isize, *mut usize);

syscall!(sys_exit, SYS_EXIT, isize);

syscall!(sys_write, SYS_WRITE, usize, *const u8, usize);

syscall!(sys_read, SYS_READ, usize, *mut u8, usize);

syscall!(sys_chdir, SYS_CHDIR, *const u8);

syscall!(sys_getcwd, SYS_GETCWD, *mut u8, usize);

syscall!(sys_openat, SYS_OPENAT, isize, *const u8, usize, usize);

syscall!(sys_getdents, SYS_GETDENTS64, usize, *mut u8, usize);

syscall!(__system_shutdown, SYSCALL_SYSTEMSHUTDOWN);

syscall!(sys_framebuffer, SYSCALL_FRAMEBUFFER);

syscall!(sys_framebuffer_flush, SYSCALL_FRAMEBUFFER_FLUSH);

syscall!(sys_event_get, SYSCALL_EVENT_GET);

syscall!(sys_listen, SYSCALL_LISTEN, *const u8);

syscall!(sys_connect, SYSCALL_CONNNET, *const u8);

syscall!(sys_close, SYS_CLOSE, usize);

syscall!(sys_get_time, SYS_GETTIMEOFDAY);
