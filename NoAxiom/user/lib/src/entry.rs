use alloc::vec::Vec;

use crate::{driver::heap, print, syscall::sys_exit};

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
    let envp = get_reg!(a2);
    if let Err(_) = heap::init() {
        panic!("heap init failed");
    };
    let mut args: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let str_start =
            unsafe { ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile() };
        let len = (0usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
            .unwrap();
        args.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }
    let mut envs: Vec<&'static str> = Vec::new();
    for i in 0.. {
        let ptr = (envp + i * core::mem::size_of::<usize>()) as *const usize;
        if ptr.is_null() {
            break;
        }
        let str_start = unsafe { ptr.read_volatile() };
        if str_start == 0 {
            break;
        }
        let len = (0usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
            .unwrap();
        envs.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }
    sys_exit(main(argc, args.as_slice(), envs.as_slice()));
}

#[linkage = "weak"]
#[no_mangle]
fn main(_argc: usize, _argv: &[&str], _envp: &[&str]) -> isize {
    unreachable!();
}
