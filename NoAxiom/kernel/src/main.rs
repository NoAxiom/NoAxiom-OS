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

// #[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;

mod arch;
mod config;
mod constant;
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
mod trap;
mod utils;

core::arch::global_asm!(include_str!("link_apps.S"));

#[no_mangle]
pub fn rust_main() {
    entry::clear_bss();
    driver::log::init();
    println!("{}", constant::banner::NOAXIOM_BANNER);
    println!("[kernel] Hello, world!");
    println!("[kernel] init memory management");
    mm::init();
    trap::init();
    println!("[kernel] push init_proc to executor");
    task::spawn_new_process(0);
    println!("[kernel] executor is running...");
    loop {
        sched::run();
    }
}
