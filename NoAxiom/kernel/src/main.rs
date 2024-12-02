//! NoAxiom main

#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(ascii_char)]
#![feature(let_chains)]
#![feature(error_in_core)]
#![feature(negative_impls)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
// #![allow(dead_code, unused_imports, unused_variables)]
// #![feature(custom_mir)]
// #![feature(core_intrinsics)]

use core::sync::atomic::{AtomicBool, Ordering};

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;

mod arch;
mod config;
mod constant;
mod cpu;
mod driver;
mod entry;
mod mm;
mod panic;
mod sched;
mod sync;
mod syscall;
mod task;
mod trap;
mod utils;

core::arch::global_asm!(include_str!("link_apps.S"));

static mut BOOT_FLAG: AtomicBool = AtomicBool::new(false);

/// boot a hardware thread
#[no_mangle]
pub fn rust_main(hart_id: usize) {
    println!("[kernel] hart id {} has been booted", hart_id);
    if unsafe {
        BOOT_FLAG
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    } {
        crate::trap::trap_init();
        println!("{}", constant::banner::NOAXIOM_BANNER);
        crate::task::spawn_new_process(0);
    } else {
    }
    loop {
        sched::run();
    }
}
