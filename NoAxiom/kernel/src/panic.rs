//! panic for no_std

use core::panic::PanicInfo;

use crate::{cpu::get_hartid, driver::sbi::shutdown, println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[kernel panic] panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
        println!("[kernel panic] hart: {}", get_hartid());
    } else {
        println!("[kernel panic (no_detail)] hart {} panicked: {}", 0, info.message().unwrap());
    }
    shutdown()
}
