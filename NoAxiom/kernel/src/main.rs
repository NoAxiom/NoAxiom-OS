//! # NoAxiom Kernel
//!
//! ## Brief
//! This is the main kernel of NoAxiom OS.
//! NoAxiom OS is an open-source operating system with Linux-like syscall
//! interfaces, which can run both on RISC-V and LoongArch architectures.
//! NoAxiom provides an async runtime with multicore asynchronous scheduling.
//! And provides asynchronous drivers when under specific platforms.
//!
//! ## Kernel Module Description
//! [`config`]: the global configs for the kernel (imports from external library)
//! [`constant`]: the constant values without any implementation
//! [`cpu`]: per-cpu structures
//! [`driver`]: the driver implementations
//! [`entry`] defines the initialize entry of NoAxiom
//! [`fs`]: the file system implementations
//! [`include`]: struct definitions with basic impls
//! [`mm`]: memory management
//! [`net`]: network stack
//! [`panic`]: panic handler
//! [`sched`]: scheduler and runtime
//! [`signal`]: signal implementations
//! [`syscall`]: syscall implementations
//! [`task`]: task management and syscall inner impls for task control block
//! [`time`]: time struct definations, utils and management
//! [`trap`]: trap handler and specific interrupt handler
//! [`utils`]: utils functions

#![no_std]
#![no_main]
#![feature(asm_const)]
#![feature(ascii_char)]
#![feature(let_chains)]
#![feature(decl_macro)]
#![feature(error_in_core)]
#![feature(negative_impls)]
#![feature(core_intrinsics)]
#![feature(naked_functions)]
#![feature(const_ptr_as_ref)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]
#![allow(internal_features)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate driver;
#[macro_use]
extern crate ksync;

use config;
mod constant;
mod cpu;
mod entry;
mod fs;
mod include;
mod io;
mod mm;
mod net;
mod panic;
mod sched;
mod signal;
mod syscall;
mod task;
mod time;
mod trap;
mod utils;

use entry::init::_boot_hart_init as main;
