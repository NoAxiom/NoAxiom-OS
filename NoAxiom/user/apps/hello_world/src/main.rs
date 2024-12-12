#![no_std]
#![no_main]

use userlib::println;

#[no_mangle]
fn main() -> i32 {
    println!("[user] hello, world!");
    0
}
