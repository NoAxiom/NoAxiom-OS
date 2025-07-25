use driver::{net::loopback::LoopBackDev, set_net_dev};

fn register_loopback_device() {
    let dev = LoopBackDev::new();
    set_net_dev(dev);
}

pub fn register_extra_devices() {
    register_loopback_device();
}
