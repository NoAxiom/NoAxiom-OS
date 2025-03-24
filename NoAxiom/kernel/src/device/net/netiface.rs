use alloc::sync::Arc;

use ksync::mutex::SpinLock;
use smoltcp::{
    iface::{Config, Interface},
    time::Instant,
    wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr},
};

use super::loopback::LoopBackDev;
use crate::{driver::net::loopback::NetDriver, time::gettime::get_time_ms};

pub struct NetIface {
    pub inner: SpinLock<Interface>,
}
impl NetIface {
    pub fn new(device: Arc<LoopBackDev>) -> Self {
        let mut iface_config = Config::new(HardwareAddress::Ethernet(EthernetAddress([
            0x02, 0x00, 0x00, 0x00, 0x00, 0x01,
        ])));
        iface_config.random_seed = 5201314;
        let mut iface = Interface::new(
            iface_config,
            &mut device,
            Instant::from_micros(get_time_ms() as i64),
        );
        iface.update_ip_addrs(|ip_addrs| {
            ip_addrs
                .push(IpCidr::new(IpAddress::v4(127, 0, 0, 1), 8))
                .unwrap();
            // ip_addrs.push(IpCidr::new(IpAddress::v6(0, 0, 0, 0, 0, 0, 0,
            // 1),128)).unwrap();
        });

        Self {
            inner: SpinLock::new(iface),
        }
    }
}
