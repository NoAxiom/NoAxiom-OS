#![no_std]
#![no_main]

use userlib::{println, syscall::{sys_exec, sys_fork, sys_yield}};

#[no_mangle]
fn main() -> i32 {
    println!("process_test: Hello, world!");
    // for i in 0..10 {
    //     println!("yield: {}", i);
    //     sys_yield();
    // }
    let pid = sys_fork();
    if pid == 0 {
        println!("child process");
        // sys_exec("\0");
    } else {
        println!("parent process");
    }
    0
}
