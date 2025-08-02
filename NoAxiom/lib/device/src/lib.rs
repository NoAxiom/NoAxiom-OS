#![no_std]
#![allow(deprecated)]
#![feature(trait_upcasting)]

extern crate alloc;

mod bus;
mod device;
mod dtb;

pub use device::{
    device_init,
    manager::{handle_irq, GeneralBus, DEV_BUS},
};
pub use dtb::{devconf, init::dtb_init};
