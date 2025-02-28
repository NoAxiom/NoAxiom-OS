#![no_std]
#![allow(unused)]
#![allow(deprecated)]

extern crate alloc;

mod varch;

#[cfg(target_arch = "loongarch64")]
mod la64;
#[cfg(target_arch = "riscv64")]
mod rv64;

pub use varch::virt_arch::VirtArch;

#[cfg(target_arch = "loongarch64")]
pub type Arch = la64::LoongArch64;
#[cfg(target_arch = "riscv64")]
pub type Arch = rv64::Riscv64;

pub type Exception = <Arch as VirtArch>::Exception;
pub type Interrupt = <Arch as VirtArch>::Interrupt;
pub type Trap = <Arch as VirtArch>::Trap;
pub type TrapContext = <Arch as VirtArch>::TrapContext;
