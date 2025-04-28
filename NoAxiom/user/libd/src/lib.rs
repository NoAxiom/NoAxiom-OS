#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(lang_items)]

extern crate alloc;

#[macro_use]
pub mod console;
pub mod arch;
pub mod entry;
pub mod errno;
mod heap;
pub mod lib_basepath;
pub mod lib_client;
pub mod lib_event;
pub mod lib_request;
pub mod lib_widget;
pub mod syscall;
pub mod utils;

pub use console::*;
use syscall::exit;

#[panic_handler]
fn _panic(info: &core::panic::PanicInfo) -> ! {
    let err = info.message().unwrap();
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{}, {}",
            location.file(),
            location.line(),
            err
        );
    } else {
        println!("Panicked: {}", err);
    }
    exit(-1)
}

#[linkage = "weak"]
#[no_mangle]
pub fn main(_: usize, _: &[&str]) -> isize {
    panic!("Cannot find main!");
}
