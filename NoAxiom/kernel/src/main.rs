//! NoAxiom main

#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(let_chains)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
// #![feature(slice_from_ptr_range)]
// #![feature(error_in_core)]
// #![feature(negative_impls)]
// #![feature(ascii_char)]
// #![allow(dead_code, unused_imports, unused_variables)]
// #![feature(custom_mir)]
// #![feature(core_intrinsics)]

#[macro_use]
extern crate arch;
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate klog;

mod entry;
mod mm;
mod panic;
mod sched;
mod syscall;
mod task;

#[no_mangle]
pub fn rust_main() -> ! {
    entry::clear_bss();
    trace!("[kernel] launched");
    println!("{}", config::NOAXIOM_BANNER);
    println!("[kernel] Hello, NoAxiom!");

    trace!("[kernel] shutdown");
    sbi::shutdown();

    // loop {
    //     sched::run();
    // }
}
