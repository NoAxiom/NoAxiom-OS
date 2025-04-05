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

extern crate alloc;
#[macro_use]
extern crate log;

pub mod address;
pub mod bss;
pub mod frame;
pub mod heap;
pub mod utils;
