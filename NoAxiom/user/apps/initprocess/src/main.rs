#![no_std]
#![no_main]

use userlib::println;

#[no_mangle]
fn main() -> i32 {
    println!("[user] initprocess has been booted!");
    0
}
