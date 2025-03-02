#![no_std]
#![allow(unused)]
#![allow(deprecated)]

extern crate alloc;

mod virt_arch;

#[cfg(target_arch = "loongarch64")]
mod la64;
#[cfg(target_arch = "riscv64")]
mod rv64;

pub use virt_arch::*;

#[cfg(target_arch = "loongarch64")]
pub type Arch = la64::LoongArch64;

#[cfg(target_arch = "riscv64")]
pub type Arch = rv64::RV64;

pub type Exception = <Arch as ArchType>::Exception;
pub type Interrupt = <Arch as ArchType>::Interrupt;
pub type Trap = <Arch as ArchType>::Trap;
pub type TrapContext = <Arch as ArchType>::TrapContext;
