#![no_std]
#![no_main]

use userlib::println;

#[no_mangle]
fn main() -> i32 {
    const END: usize = 1e7 as usize;
    let mut sum: usize = 0;
    for i in 0..END {
        if (i & 1) == 0 {
            sum += i;
        }
    }
    println!("long loop: answer is {}", sum);
    0
}
