#![no_std]
#![feature(fn_align)]
#![feature(asm_const)]
#![feature(ascii_char)]
#![feature(decl_macro)]
#![feature(let_chains)]
#![feature(lang_items)]
#![feature(error_in_core)]
#![feature(negative_impls)]
#![feature(core_intrinsics)]
#![feature(naked_functions)]
#![feature(sync_unsafe_cell)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
#![allow(internal_features)]
#![allow(deprecated)]

extern crate alloc;
#[macro_use]
extern crate log;

/// common modules
/// this mod includes all common traits for arch
mod common;

/// utilities for private use
/// includes console, address and other macros
#[macro_use]
mod utils;

/// looarch64 specific mod
/// this implements all common traits for loongarch64
#[cfg(target_arch = "loongarch64")]
mod la64;

/// riscv64 specific mod
/// this implements all common traits for riscv64
#[cfg(target_arch = "riscv64")]
mod rv64;

pub use common::*;
#[cfg(target_arch = "loongarch64")]
pub use la64::la_libc_import::*;

/// [`Arch`] is designed to be used as a trait holder
/// currently it's defined as [`la64::LA64`] for loongarch64
#[cfg(target_arch = "loongarch64")]
pub type Arch = la64::LA64;

/// [`Arch`] is designed to be used as a trait holder
/// currently it's defined as [`rv64::RV64`] for riscv64
#[cfg(target_arch = "riscv64")]
pub type Arch = rv64::RV64;

/// [`TrapContext`] represents for the trap context from current arch
pub type TrapContext = <Arch as ArchTrap>::TrapContext;

/// [`VirtPageTable`] represents for the page table from current arch
/// note that it won't contain any physical memory assignment,
/// and the kernel should handle memory menagement manually
pub type VirtPageTable = <Arch as ArchMemory>::PageTable;

/// [`PageTableEntry`] represents for the page table entry from current arch
pub type PageTableEntry = <<Arch as ArchMemory>::PageTable as ArchPageTable>::PageTableEntry;

#[cfg(target_arch = "loongarch64")]
pub use la64::{_entry, _entry_other_hart};
#[cfg(target_arch = "riscv64")]
pub use rv64::{_entry, _entry_other_hart};
