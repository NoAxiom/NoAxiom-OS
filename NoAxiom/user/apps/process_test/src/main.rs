#![no_std]
#![no_main]

use userlib::{println, syscall::sys_yield};

#[no_mangle]
fn main() -> i32 {
    println!("process_test: Hello, world!");
    for i in 0..10 {
        println!("yield: {}", i);
        sys_yield();
    }
    0
}
