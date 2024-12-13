//! panic for no_std

use core::{panic::PanicInfo, ptr};

use crate::{cpu::get_hartid, driver::sbi::shutdown, println};

#[no_mangle]
pub fn backtrace() {
    println!("[kernel] rqdmap: 123");
    let mut fp: *const usize;
    unsafe {
        core::arch::asm!("mv {}, fp", out(reg) fp);

        println!("[Kernel] == Begin stack trace ==");
        while fp != ptr::null() {
            let ra = *fp.sub(1);
            let prev_fp = *fp.sub(2) as *const usize;
            println!(
                "[kernel] 0x{:x}, fp = 0x{:x}",
                ra as usize, prev_fp as usize
            );
            fp = prev_fp;
        }
        println!("[Kernel] == End stack trace ==");
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        debug!("panic!!!");
        // backtrace();
        println!(
            "[kernel panic] panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
        println!("[kernel panic] hart: {}", get_hartid());
    } else {
        println!(
            "[kernel panic (no_detail)] hart {} panicked: {}",
            get_hartid(),
            info.message().unwrap()
        );
    }
    loop {}
    shutdown()
}
