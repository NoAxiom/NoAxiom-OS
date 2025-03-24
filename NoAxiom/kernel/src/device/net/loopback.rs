use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use ksync::mutex::SpinLock;
use smoltcp::{
    iface::{self, Config, Interface},
    phy::{RxToken, TxToken},
    time::Instant,
    wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr},
};

use super::{NetDevice, NET_IFACE};
use crate::{
    syscall::SysResult,
    time::gettime::{get_time, get_time_ms},
};

pub type LoopBackDev = smoltcp::phy::Loopback;

impl NetDevice for LoopBackDev {
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

    fn inner_iface(&self) -> &SpinLock<Interface> {
        &NET_IFACE.get().unwrap().inner
    }
}
