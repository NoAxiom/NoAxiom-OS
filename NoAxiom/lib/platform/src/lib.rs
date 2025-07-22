#![no_std]
#![allow(deprecated)]

extern crate alloc;

#[cfg(all(target_arch = "loongarch64", feature = "board"))]
mod loongarch64_board;
#[cfg(all(target_arch = "loongarch64", feature = "qemu"))]
mod loongarch64_qemu;
#[cfg(all(target_arch = "riscv64", feature = "board"))]
mod riscv64_board;
#[cfg(all(target_arch = "riscv64", feature = "qemu"))]
mod riscv64_qemu;

#[cfg(all(target_arch = "loongarch64", feature = "board"))]
pub use loongarch64_board::*;
#[cfg(all(target_arch = "loongarch64", feature = "qemu"))]
pub use loongarch64_qemu::*;
#[cfg(all(target_arch = "riscv64", feature = "board"))]
pub use riscv64_board::*;
#[cfg(all(target_arch = "riscv64", feature = "qemu"))]
pub use riscv64_qemu::*;

pub mod archs;
pub mod dtb;
