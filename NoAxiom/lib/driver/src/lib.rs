#![no_std]
#![allow(deprecated)]
#![feature(impl_trait_in_assoc_type)]

use crate::{
    archs::arch_driver_init,
    bus::bus_init,
    devices::{block::block_init, net::net_init},
};
extern crate alloc;

mod archs;
mod bus;
pub mod devices;
mod irq;
mod macros;

pub use devices::manager::*;
pub use irq::handle_irq;

pub fn driver_init() {
    bus_init();
    block_init();
    net_init();
    arch_driver_init();
}
