#![no_std]
#![no_main]

extern crate alloc;

use libd::syscall::{execve, fork, wait, yield_};

#[no_mangle]
fn main() -> i32 {
    // 用户态执行fork，execve系统调用的请求
    if fork() == 0 {
        execve(
            "busybox\0",
            &[
                "busybox\0".as_ptr(),
                "sh\0".as_ptr(),
                core::ptr::null::<u8>(),
            ],
            &[
                "PATH=/glibc\0".as_ptr(),
                "LD_LIBRARY_PATH=/glibc\0".as_ptr(),
                "TERM=screen\0".as_ptr(),
                core::ptr::null::<u8>(),
            ],
        );
    } else {
        loop {
            let mut exit_code: usize = 0;
            let tid = wait(-1, &mut exit_code);
            if tid == -1 {
                yield_();
                continue;
            } else {
                break;
            }
        }
    }
    0
}
