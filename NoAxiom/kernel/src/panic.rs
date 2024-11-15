//! panic for no_std

use core::panic::PanicInfo;

use crate::{cpu::hartid, driver::sbi::shutdown, println};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[kernel] hart {} panicked at {}:{} {}",
            hartid(),
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("[kernel] hart {} panicked: {}", 0, info.message().unwrap());
    }
    shutdown()
}
