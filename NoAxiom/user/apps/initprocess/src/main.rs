#![no_std]
#![no_main]

use userlibs::{print, println};

#[no_mangle]
fn main() -> i32 {
    print!("initprocess\n");
    println!("todo: initprocess");
    0
}
