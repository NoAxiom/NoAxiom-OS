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
#![allow(deprecated)]
// #![allow(dead_code, unused_imports, unused_variables)]
// #![feature(custom_mir)]
// #![feature(core_intrinsics)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate console;

mod config;
mod constant;
mod cpu;
mod device;
mod driver;
mod entry;
mod fs;
mod include;
mod ipc;
mod mm;
mod panic;
mod platform;
mod sched;
mod syscall;
mod task;
mod time;
mod trap;
mod utils;

core::arch::global_asm!(include_str!("link_apps.S"));

/// rust_main: only act as a task runner
/// called by [`entry::init::boot_hart_init`]
#[no_mangle]
pub fn rust_main() {
    trace!("token {:#x}", crate::mm::page_table::current_token());
    info!("[kernel] hart id {} has been booted", cpu::get_hartid());
    trap::ipi::send_ipi_test();
    loop {
        sched::executor::RUNTIME.run();
    }
}
