#![no_std]
#![no_main]

mod drv_ahci;
mod libahci;
mod libata;
mod platform;

pub use libahci::AhciDevice;
