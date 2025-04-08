//! panic for no_std

use core::panic::PanicInfo;

use arch::{Arch, ArchBoot};

use crate::cpu::get_hartid;

lazy_static::lazy_static! {
    static ref PANIC_FLAG: spin::Mutex<bool> = spin::Mutex::new(false);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[panic] panicked at {}:{}\n[panic] HART{}, {}",
            location.file(),
            location.line(),
            get_hartid(),
            info.message().unwrap(),
        );
    } else {
        println!(
            "[panic (no_detail), HART{}] panicked: {}",
            get_hartid(),
            info.message().unwrap()
        );
    }
    Arch::arch_info_print();
    loop {}
    // platform::shutdown()
}
