use driver::net::loopback::LoopBackDev;

use crate::device::manager::DEV_BUS;

fn register_loopback_device() {
    let dev = LoopBackDev::new();
    DEV_BUS.add_network_device(dev);
}

pub fn register_extra_devices() {
    register_loopback_device();
}
