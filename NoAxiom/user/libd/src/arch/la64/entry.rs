use alloc::vec::Vec;
use core::arch::asm;

use crate::{heap, main, syscall::exit};

#[linkage = "weak"]
#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe extern "C" fn _start() -> ! {
    let argc: usize;
    let argv: usize;
    unsafe {
        asm!(
            "ld.d $a0, $sp, 16",
            "addi.d $a1, $sp, 24",
            out("$a0") argc,
            out("$a1") argv
        );
    }
    if let Err(_) = heap::init() {
        panic!("heap init failed");
    };
    let mut v: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let str_start =
            unsafe { ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile() };
        let len = (0usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
            .unwrap();
        v.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }
    exit(main(argc, v.as_slice()));
}
