#![no_std]
#![allow(deprecated)]

use arch::{Arch, ArchMemory};

extern crate alloc;

mod platform;

pub fn init(dtb: usize) {
    let platfrom_info = platform::platform_init(dtb);
    platform::plic::init_plic(platfrom_info.plic.start | Arch::KERNEL_ADDR_OFFSET);
    // driver::init::driver_init();
    platform::plic::register_to_hart();
}
