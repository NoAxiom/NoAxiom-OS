//! panic for no_std

use core::panic::PanicInfo;

use crate::{cpu::get_hartid, driver::sbi::shutdown, println};

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
    shutdown()
}
