//! NoAxiom main

#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(let_chains)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
// #![feature(error_in_core)]
// #![feature(negative_impls)]
// #![feature(ascii_char)]
// #![allow(dead_code, unused_imports, unused_variables)]
// #![feature(custom_mir)]
// #![feature(core_intrinsics)]

extern crate alloc;

mod arch;
mod config;
#[macro_use]
mod cpu;
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
    println!("[kernel] Hello, world!");
    println!("{}", config::NOAXIOM_BANNER);
    println!("[kernel] init memory management...");
    mm::init();
    println!("[kernel] executor is running...");
    driver::sbi::shutdown();
    // loop {
    //     sched::executor::run();
    // }
}
