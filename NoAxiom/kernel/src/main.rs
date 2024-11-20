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

core::arch::global_asm!(include_str!("link_apps.S"));

async fn temp_test_task() {
    println!("[kernel] read_user_task is running...");
    let mut task = task::Task::new(0).await;
}

#[no_mangle]
pub fn rust_main() {
    entry::clear_bss();
    driver::log::init();
    println!("{}", config::NOAXIOM_BANNER);
    println!("[kernel] Hello, world!");

    println!("[kernel] init memory management");
    mm::init();
    mm::remap_test();

    println!("[kernel] push init_proc to executor");
    sched::spawn_utask(alloc::sync::Arc::from(crate::task::Task {
        tid: crate::task::tid_alloc(),
        debug_message: alloc::string::String::from("[kernel] hello world from kernel"),
    }));

    sched::spawn_raw(temp_test_task());

    println!("[kernel] executor is running...");
    loop {
        sched::run();
    }
}
