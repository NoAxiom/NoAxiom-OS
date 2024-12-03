macro_rules! syscall_id {
    ($name:ident, $val:expr) => {
        const $name: usize = $val;
    };
}

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

#[inline(always)]
fn syscall(id: usize, args: [usize; 6]) -> isize {
    let mut ret: isize;
    // 通过汇编指令描述了具体用哪些寄存器来保存参数和返回值
    // 返回内核态后，通过系统调用的请求从寄存器中取得相应的值并执行相应系统调用
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x13") args[3],
            in("x14") args[4],
            in("x15") args[5],
            in("x17") id
        );
    }
    ret
}

pub fn sys_write(fd: usize, buf_addr: usize, buf_len: usize) -> isize {
    syscall(SYS_WRITE, [fd, buf_addr, buf_len, 0, 0, 0])
}

pub fn sys_exit(exit_code: isize) -> ! {
    syscall(SYS_EXIT, [exit_code as usize, 0, 0, 0, 0, 0]);
    loop {}
}
