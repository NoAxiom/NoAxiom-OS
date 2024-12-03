#![no_std]
#![no_main]

use userlib::{print, println};

#[no_mangle]
fn main() -> i32 {
    println!("[user] hello, world!\n");
    0
}
