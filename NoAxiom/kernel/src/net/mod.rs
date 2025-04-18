use alloc::{collections::btree_map::BTreeMap, sync::Arc};

use driver::devices::impls::net::NetWorkDev;
use ksync::mutex::RwLock;
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
    pub static ref PORT_MANAGER: PortManager = PortManager::new();
    pub static ref NET_DEVICES: RwLock<BTreeMap<usize, Arc<dyn NetWorkDev>>> = RwLock::new(BTreeMap::new());
}
