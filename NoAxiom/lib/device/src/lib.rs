#![no_std]
#![allow(deprecated)]

extern crate alloc;

mod archs;
mod bus;
mod device;

pub use archs::*;
pub use device::basic::device_init;
