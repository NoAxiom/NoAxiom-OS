//! panic for no_std

use core::{arch::asm, panic::PanicInfo, ptr};

use crate::{cpu::get_hartid, utils::symbol_table::SYMBOL_TABLE};

lazy_static::lazy_static! {
    static ref PANIC_FLAG: spin::Mutex<bool> = spin::Mutex::new(false);
}

fn resolve_symbol(addr: usize) -> Option<&'static str> {
    for &(sym_addr, sym_name) in SYMBOL_TABLE {
        if addr == sym_addr {
            return Some(sym_name);
        }
    }
    None
}

pub unsafe fn print_backtrace() -> () {
    let mut fp: *const usize;
    asm!("mv {}, fp", out(reg) fp);

    if *PANIC_FLAG.lock() {
        error!("Panic occurred during panic handling, aborting");
        sbi_rt::legacy::shutdown();
    }
    *PANIC_FLAG.lock() = true;

    println!("=============== Begin stack trace ===============");
    while fp != ptr::null() {
        let saved_ra = *fp.sub(1);
        let saved_fp = *fp.sub(2);

        if let Some(name) = resolve_symbol(saved_ra) {
            println!("  0x {:x} ({})", saved_ra, name);
        } else {
            println!("  0x {:x}", saved_ra);
        }

        fp = saved_fp as *const usize;
    }
    println!("=============== End stack trace ===============");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[panic, HART {}] panicked at {}:{} {}",
            get_hartid(),
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!(
            "[panic (no_detail), HART {}] panicked: {}",
            get_hartid(),
            info.message().unwrap()
        );
    }
    unsafe {
        print_backtrace();
    }
    sbi_rt::legacy::shutdown()
}
