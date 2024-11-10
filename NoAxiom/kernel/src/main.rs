//! NoAxiom main

#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
#![feature(error_in_core)]
#![feature(sync_unsafe_cell)]
#![feature(negative_impls)]
#![feature(ascii_char)]
#![feature(let_chains)]
// #![allow(dead_code, unused_imports)]
// #![allow(unused_variables)]
// #![feature(custom_mir)]
// #![feature(core_intrinsics)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate klog;
#[macro_use]
extern crate arch;

mod entry;
mod panic;
mod sched;
mod syscall;
mod task;

#[no_mangle]
pub fn rust_main() -> ! {
    entry::clear_bss();
    println!("{}", config::NOAXIOM_BANNER);
    println!("[kernel] Hello, NoAxiom!");
    loop {
        sbi::shutdown()
    }
}
