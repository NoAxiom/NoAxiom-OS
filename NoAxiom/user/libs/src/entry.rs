use alloc::vec::Vec;

use crate::{driver::heap, syscall::sys_exit};

macro_rules! get_reg {
    ($reg:ident) => {{
        let reg: usize;
        unsafe {
            core::arch::asm! {
                concat!("mv {0}, ", stringify!($reg)),
                out(reg) reg,
            }
        }
        reg
    }};
}

#[linkage = "weak"]
#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe extern "C" fn _start() -> ! {
    let argc = get_reg!(a0);
    let argv = get_reg!(a1);
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
    sys_exit(main(argc, v.as_slice()))
}

#[linkage = "weak"]
#[no_mangle]
fn main(_: usize, _: &[&str]) -> isize {
    unreachable!();
}
