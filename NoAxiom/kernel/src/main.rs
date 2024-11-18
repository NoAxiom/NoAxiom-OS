//! NoAxiom main

#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(let_chains)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
// #![allow(dead_code, unused_imports, unused_variables)]
// #![feature(error_in_core)]
// #![feature(negative_impls)]
// #![feature(ascii_char)]
// #![feature(custom_mir)]
// #![feature(core_intrinsics)]

use log::info;

extern crate alloc;

mod arch;
mod config;
#[macro_use]
mod driver;
mod entry;
mod mm;
mod panic;
mod sched;
mod sync;
mod syscall;
mod task;

#[no_mangle]
pub fn rust_main() {
    entry::clear_bss();
    driver::log::init();
    info!("{}", config::NOAXIOM_BANNER);
    info!("[kernel] Hello, world!");

    println!("[kernel] init memory management");
    mm::init();

    println!("[kernel] push init_proc to executor");
    sched::spawn_utask(alloc::sync::Arc::from(crate::task::Task {
        debug_message: alloc::string::String::from("hello world from test_task"),
    }));

    println!("[kernel] executor is running...");
    sched::run();
    driver::sbi::shutdown();
}
