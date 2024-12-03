#![no_std]
#![no_main]

use userlib::{print, println};

#[no_mangle]
fn main() -> i32 {
    print!("initprocess\n");
    println!("todo: initprocess");
    0
}
