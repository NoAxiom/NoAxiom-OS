use alloc::{collections::btree_map::BTreeMap, sync::Arc};

use driver::devices::impls::net::NetWorkDev;
use ksync::mutex::{RwLock, SpinLock};
use port_manager::PortManager;
use socket_set::SocketSet;

mod port_manager;
mod socket;
mod socket_set;
pub mod socketfile;
mod tcpsocket;
mod udpsocket;

lazy_static::lazy_static! {
    pub static ref SOCKET_SET: SocketSet = SocketSet::new();
    pub static ref TCP_PORT_MANAGER: Arc<SpinLock<PortManager>> = Arc::new(SpinLock::new(PortManager::new()));
    pub static ref UDP_PORT_MANAGER: Arc<SpinLock<PortManager>> = Arc::new(SpinLock::new(PortManager::new()));
    pub static ref NET_DEVICES: RwLock<BTreeMap<usize, Arc<&'static dyn NetWorkDev>>> = {
        let net_devices = RwLock::new(BTreeMap::new());
        net_devices.write().insert(0, driver::get_net_dev());
        net_devices
    };
}
