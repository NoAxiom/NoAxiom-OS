//! NoAxiom main

#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(ascii_char)]
#![feature(let_chains)]
#![feature(decl_macro)]
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
mod mm;
// mod net;
mod panic;
mod platform;
mod sched;
mod signal;
mod syscall;
mod task;
mod time;
mod trap;
mod utils;

core::arch::global_asm!(include_str!("link_apps.S"));

/// rust_main: only act as a task runner
/// called by [`entry::init::_boot_hart_init`]
#[no_mangle]
pub fn rust_main() {
    info!("[kernel] hart id {} has been booted", cpu::get_hartid());
    loop {
        sched::runtime::run();
    }
}
