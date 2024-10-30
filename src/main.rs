#![no_std]
#![no_main]

mod entry;
mod language_items;
mod sbi;
mod syscall;
mod utils;

core::arch::global_asm!(include_str!("entry/entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    entry::clear_bss();
    println!("Hello, world from rust_main!");
    sbi::shutdown();
}
