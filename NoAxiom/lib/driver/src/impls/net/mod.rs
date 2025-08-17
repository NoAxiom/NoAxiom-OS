use alloc::string::String;

use ksync::mutex::SpinLock;
use smoltcp::{
    iface::{Interface, SocketSet},
    wire::EthernetAddress,
};

use crate::basic::{DevResult, Device};

pub mod loopback;
pub mod ls2k1000_gmac;
mod utils;

pub trait NetWorkDevice: Send + Sync + Device {
    /// get the MAC address of the network card
    fn mac(&self) -> EthernetAddress;

    fn iface_name(&self) -> String;

    /// get the network card ID
    fn nic_id(&self) -> usize;

    fn poll(&self, sockets: &mut SocketSet) -> DevResult<()>;

    // fn update_ip_addrs(&self, ip_addrs: &[wire::IpCidr]) -> SysResult<()>;

    /// get the smoltcp interface type
    fn inner_iface(&self) -> &SpinLock<Interface>;
    // fn as_any_ref(&'static self) -> &'static dyn core::any::Any;

    // fn addr_assign_type(&self) -> u8;

    // fn net_device_type(&self) -> u16;

    // fn net_state(&self) -> NetDeivceState;

    // fn set_net_state(&self, state: NetDeivceState);

    // fn operstate(&self) -> Operstate;

    // fn set_operstate(&self, state: Operstate);
}
