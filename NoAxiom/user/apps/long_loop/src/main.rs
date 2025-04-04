#![no_std]
#![no_main]

use userlib::println;

#[no_mangle]
fn main() -> i32 {
    const END: usize = 1e8 as usize;
    let mut sum: usize = 0;
    for i in 0..END {
        if i % (END / 100) == 0 {
            println!("{}", i * 100 / END);
        }
        if (i & 1) == 0 {
            sum += i;
        }
    }
    println!("long loop: answer is {}", sum);
    0
}
