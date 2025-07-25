#![no_std]
#![allow(deprecated)]
#![feature(trait_upcasting)]

extern crate alloc;

mod archs;
mod bus;
mod device;

pub use archs::*;
pub use device::{
    basic::{device_init, handle_irq},
    manager::{GeneralBus, DEV_BUS},
};
