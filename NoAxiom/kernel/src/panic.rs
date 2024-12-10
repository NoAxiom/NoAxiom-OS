//! panic for no_std

use core::panic::PanicInfo;

use crate::{cpu::get_hartid, driver::sbi::shutdown, println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[kernel panic] hart {} panicked at {}:{} {}",
            get_hartid(),
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("[kernel panic] hart {} panicked: {}", 0, info.message().unwrap());
    }
    shutdown()
}
