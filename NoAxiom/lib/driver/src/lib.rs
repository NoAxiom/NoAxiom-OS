#![no_std]
#![allow(deprecated)]

mod bus;
mod devices;
mod dtb;

extern crate alloc;

pub fn init(dtb: usize) {
    dtb::init(dtb);
    bus::probe_bus();
}
