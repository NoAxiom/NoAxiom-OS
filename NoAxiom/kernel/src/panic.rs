//! panic for no_std

use core::panic::PanicInfo;

use arch::{Arch, ArchBoot, ArchSbi};

use crate::{cpu::get_hartid, mm::map_area::MAP_ADDRESS};

lazy_static::lazy_static! {
    static ref PANIC_FLAG: spin::Mutex<bool> = spin::Mutex::new(false);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debug!("[panic] DEBUG_ADDR: {:#x}", unsafe { MAP_ADDRESS });
    Arch::arch_info_print();
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
    // loop {}
    Arch::shutdown()
}
