#![no_std]
#![feature(asm_const)]
#![feature(ascii_char)]
#![feature(decl_macro)]
#![feature(let_chains)]
#![feature(lang_items)]
#![feature(error_in_core)]
#![feature(negative_impls)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
#![allow(internal_features)]
#![allow(deprecated)]

extern crate alloc;

#[macro_use]
mod common;

#[macro_use]
mod console;

#[cfg(target_arch = "loongarch64")]
mod la64;
#[cfg(target_arch = "riscv64")]
mod rv64;
mod utils;

pub use common::*;
#[cfg(target_arch = "loongarch64")]
pub use la64::la_libc_import::*;

#[cfg(target_arch = "loongarch64")]
pub type Arch = la64::LA64;
#[cfg(target_arch = "riscv64")]
pub type Arch = rv64::RV64;
pub type TrapContext = <Arch as ArchTrap>::TrapContext;
pub type VirtPageTable = <Arch as ArchMemory>::PageTable;
pub type PageTableEntry = <<Arch as ArchMemory>::PageTable as ArchPageTable>::PageTableEntry;

#[cfg(target_arch = "loongarch64")]
pub use la64::{_entry, _entry_other_hart};
#[cfg(target_arch = "riscv64")]
pub use rv64::{_entry, _entry_other_hart};
