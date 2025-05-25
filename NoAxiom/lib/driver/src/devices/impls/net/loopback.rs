use alloc::string::String;
use core::ops::DerefMut;

use arch::{Arch, ArchTime};
use include::errno::Errno;
use ksync::mutex::SpinLock;
use smoltcp::{
    iface::{self, Config, Interface},
    phy::{Device, Loopback, Medium},
    time::Instant,
    wire::{EthernetAddress, IpAddress, IpCidr},
};

use super::NetWorkDev;
use crate::devices::impls::device::DevResult;

pub struct LoopBackDev {
    // todo: use one lock
    pub interface: SpinLock<Interface>,
    pub dev: SpinLock<Loopback>,
}

impl LoopBackDev {
    pub fn new() -> Self {
        let mut device = Loopback::new(Medium::Ethernet);
        let iface = {
            let mut config = match device.capabilities().medium {
                Medium::Ethernet => {
                    Config::new(EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]).into())
                }
                Medium::Ip => Config::new(smoltcp::wire::HardwareAddress::Ip),
            };
            config.random_seed = 0x9898998;
            let mut iface = Interface::new(
                config,
                &mut device,
                Instant::from_millis(get_time_ms() as i64),
            );
            iface.update_ip_addrs(|ip_addrs| {
                ip_addrs
                    .push(IpCidr::new(IpAddress::v4(127, 0, 0, 1), 24))
                    .unwrap();
                ip_addrs
                    .push(IpCidr::new(IpAddress::v6(0, 0, 0, 0, 0, 0, 0, 1), 128))
                    .unwrap();
            });
            let gate_way = IpAddress::v4(127, 0, 0, 1);
            match gate_way {
                IpAddress::Ipv4(v4) => iface.routes_mut().add_default_ipv4_route(v4).unwrap(),
                IpAddress::Ipv6(_) => todo!(),
            };
            iface
        };
        Self {
            interface: SpinLock::new(iface),
            dev: SpinLock::new(device),
        }
    }
}

impl NetWorkDev for LoopBackDev {
    fn mac(&self) -> EthernetAddress {
        EthernetAddress([0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
    }

    fn iface_name(&self) -> String {
        String::from("lo")
    }

    fn nic_id(&self) -> usize {
        // loopback's netcard id is 0
        0
    }

    fn poll(&self, sockets: &mut iface::SocketSet) -> DevResult<()> {
        let mut iface = self.interface.lock();
        let mut device_guard = self.dev.lock();
        let device = device_guard.deref_mut();
        let res = iface.poll(Instant::from_millis(get_time_ms() as i64), device, sockets);
        if res {
            // log::info!("[LoopBackDev::poll] polled {res}");
            Ok(())
        } else {
            Err(Errno::EAGAIN)
        }
    }

    fn inner_iface(&self) -> &SpinLock<Interface> {
        &self.interface
    }
}

pub fn get_time_ms() -> usize {
    const MSEC_PER_SEC: usize = 1000;
    arch::Arch::get_time() / (Arch::get_freq() / MSEC_PER_SEC)
}
