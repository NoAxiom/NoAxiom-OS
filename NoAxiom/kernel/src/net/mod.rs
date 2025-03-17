use port_manager::PortManager;
use socket_set::SocketSet;

mod port_manager;
mod socket;
mod socket_set;
mod tcpsocket;

lazy_static::lazy_static! {
    pub static ref SOCKET_SET: SocketSet = SocketSet::new();
    pub static ref PORT_MANAGER: PortManager = PortManager::new();
}
