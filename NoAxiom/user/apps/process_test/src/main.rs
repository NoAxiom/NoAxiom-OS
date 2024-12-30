#![no_std]
#![no_main]

use userlib::{
    println,
    syscall::{sys_exec, sys_fork, sys_yield},
};

#[no_mangle]
fn main() -> i32 {
    println!("process_test: Hello, world!");
    let pid = sys_fork();
    if pid == 0 {
        println!("child process0, exec long_loop");
        sys_exec("long_loop\0");
    }
    sys_fork();
    let pid = sys_fork();
    if pid == 0 {
        // sys_exec("hello_world\0");
        // println!("ERROR!!! unreachable: child process1");
        println!("child process1");
    } else {
        println!("parent process1");
    }
    let pid = sys_fork();
    if pid == 0 {
        println!("child process2");
    } else {
        // sys_exec("ktest\0");
        // println!("ERROR!!! unreachable: parent process2");
        println!("parent process2, do yield");
        for i in 0..10 {
            println!("yield: {}", i);
            sys_yield();
        }
    }
    let pid = sys_fork();
    if pid == 0 {
        println!("child process3");
    } else {
        // sys_exec("long_loop\0");
        // println!("ERROR!!! unreachable: parent process3");
        println!("parent process3");
    }
    0
}
