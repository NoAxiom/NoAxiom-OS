use alloc::string::String;
use core::ops::DerefMut;

use include::errno::Errno;
use ksync::mutex::SpinLock;
use smoltcp::{
    iface::{Config, Interface},
    wire::{EthernetAddress, IpAddress, IpCidr},
};

use crate::{
    basic::{Device, DeviceTreeInfo, DeviceType, NetDeviceType},
    net::{
        ls2k1000_gmac::{drv_eth::eth_init, eth_defs::LsGmacInner},
        utils::get_time_instant,
        NetWorkDevice,
    },
    probe::basic::DeviceConfigType,
};

pub struct LsGmacDevice {
    interface: SpinLock<Interface>,
    dev: SpinLock<LsGmacInner>,
}

impl Device for LsGmacDevice {
    fn device_name(&self) -> &'static str {
        "LS2K1000 GMAC Device"
    }
}

// 可调用的驱动接口：
// eth_init
// eth_tx
// eth_rx
// eth_irq

impl NetWorkDevice for LsGmacDevice {
    fn iface_name(&self) -> String {
        String::from("LS2K1000 GMAC")
    }
    fn inner_iface(&self) -> &ksync::mutex::SpinLock<smoltcp::iface::Interface> {
        &self.interface
    }
    fn mac(&self) -> EthernetAddress {
        EthernetAddress::from_bytes(&self.dev.lock().MacAddr)
    }
    fn nic_id(&self) -> usize {
        0
    }
    fn poll(&self, sockets: &mut smoltcp::iface::SocketSet) -> crate::basic::DevResult<()> {
        let mut iface = self.interface.lock();
        let mut device_guard = self.dev.lock();
        let device = device_guard.deref_mut();
        let res = iface.poll(get_time_instant(), device, sockets);
        if res {
            // log::info!("[LoopBackDev::poll] polled {res}");
            Ok(())
        } else {
            Err(Errno::EAGAIN)
        }
    }
}

impl LsGmacDevice {
    pub fn new(base_addr: usize) -> Option<Self> {
        if base_addr != 0x40040000 {
            return None;
        }
        let mut device: LsGmacInner = unsafe { core::mem::zeroed() };
        device.iobase = base_addr as u64;
        eth_init(&mut device);

        let iface = {
            let mut config = Config::new(EthernetAddress(device.MacAddr).into());
            config.random_seed = 0x9898998;
            let mut iface = Interface::new(config, &mut device, get_time_instant());
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

        Some(Self {
            interface: SpinLock::new(iface),
            dev: SpinLock::new(device),
        })
    }
}

impl DeviceTreeInfo for LsGmacDevice {
    const DEVICE_CONFIG_TYPE: &'static DeviceConfigType = &DeviceConfigType::DeviceTree;
    const OF_TYPE: &'static str = "snps,dwmac-3.70a";
    const DEVICE_TYPE: &'static DeviceType = &DeviceType::Net(NetDeviceType::LsGmac);
}

mod smoltcp_impl {
    use alloc::vec::Vec;

    use smoltcp::{
        phy::{Device as SmoltcpDevice, DeviceCapabilities, RxToken, TxToken},
        time::Instant,
    };

    use crate::net::ls2k1000_gmac::{
        drv_eth::{eth_rx, eth_tx, eth_tx_can_send},
        eth_defs::LsGmacInner,
    };

    pub struct LsGmacTxToken<'a> {
        pub device: &'a mut LsGmacInner,
    }

    impl<'a> TxToken for LsGmacTxToken<'a> {
        fn consume<R, F>(self, len: usize, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R,
        {
            let mut buffer = vec![0u8; len];
            let result = f(buffer.as_mut_slice());
            eth_tx(self.device, buffer.as_mut_slice());
            return result;
        }
    }

    pub struct LsGmacRxToken {
        pub buffer: Vec<u8>,
    }

    impl RxToken for LsGmacRxToken {
        fn consume<R, F>(mut self, f: F) -> R
        where
            F: FnOnce(&mut [u8]) -> R,
        {
            let result = f(self.buffer.as_mut_slice());
            return result;
        }
    }

    impl SmoltcpDevice for LsGmacInner {
        type RxToken<'a> = LsGmacRxToken;
        type TxToken<'a> = LsGmacTxToken<'a>;
        fn capabilities(&self) -> DeviceCapabilities {
            let res = {
                let mut capabilities = DeviceCapabilities::default();
                capabilities.max_transmission_unit = 65535;
                capabilities.medium = smoltcp::phy::Medium::Ethernet;
                capabilities
            };
            res
        }
        fn receive(
            &mut self,
            _timestamp: Instant,
        ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
            match eth_rx(self) {
                Some(buffer) => {
                    let rx = LsGmacRxToken { buffer };
                    let tx = LsGmacTxToken { device: self };
                    Some((rx, tx))
                }
                None => {
                    return None;
                }
            }
        }
        fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
            if eth_tx_can_send(self) {
                return None;
            } else {
                let tx = LsGmacTxToken { device: self };
                Some(tx)
            }
        }
    }
}
