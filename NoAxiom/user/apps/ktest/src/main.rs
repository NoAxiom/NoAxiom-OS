#![no_std]
#![no_main]

use userlibs::println;

#[no_mangle]
fn main() -> i32 {
    println!("[user] test for kernel ====================\n");
    0
}
