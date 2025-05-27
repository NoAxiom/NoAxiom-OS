use alloc::{collections::btree_map::BTreeMap, sync::Arc};

use driver::devices::impls::net::NetWorkDev;
use handle::HandleItem;
use ksync::mutex::{RwLock, SpinLock};
use port_manager::PortManager;
use smoltcp::iface::SocketHandle;
use socket_set::SocketSet;

mod handle;
mod poll;
mod port_manager;
mod socket;
mod socket_set;
pub mod socketfile;
mod tcpsocket;
mod udpsocket;

lazy_static::lazy_static! {
    pub static ref SOCKET_SET: SocketSet = SocketSet::new();
    pub static ref HANDLE_MAP: RwLock<BTreeMap<SocketHandle, HandleItem>> = RwLock::new(BTreeMap::new());
    pub static ref TCP_PORT_MANAGER: Arc<SpinLock<PortManager>> = Arc::new(SpinLock::new(PortManager::new()));
    pub static ref UDP_PORT_MANAGER: Arc<SpinLock<PortManager>> = Arc::new(SpinLock::new(PortManager::new()));
    pub static ref NET_DEVICES: RwLock<BTreeMap<usize, Arc<&'static dyn NetWorkDev>>> = {
        let net_devices = RwLock::new(BTreeMap::new());
        net_devices.write().insert(0, driver::get_net_dev());
        net_devices
    };
}

#[allow(dead_code)]
pub mod test {
    use core::ops::DerefMut;

    use driver::devices::impls::net::loopback::LoopBackDev;
    use ksync::mutex::SpinLock;
    use smoltcp::{iface::SocketSet, socket::tcp, time::Instant};

    use crate::time::gettime::get_time_ms;

    /// 元数据的缓冲区的大小
    pub const DEFAULT_METADATA_BUF_SIZE: usize = 1024;
    /// 默认的接收缓冲区的大小 receive
    pub const DEFAULT_RX_BUF_SIZE: usize = 512 * 1024;
    /// 默认的发送缓冲区的大小 transmiss
    pub const DEFAULT_TX_BUF_SIZE: usize = 512 * 1024;

    fn create_new_socket() -> tcp::Socket<'static> {
        // 初始化tcp的buffer
        let rx_buffer = tcp::SocketBuffer::new(vec![0; DEFAULT_RX_BUF_SIZE]);
        let tx_buffer = tcp::SocketBuffer::new(vec![0; DEFAULT_TX_BUF_SIZE]);
        tcp::Socket::new(rx_buffer, tx_buffer)
    }

    pub fn net_test() {
        let loopback = LoopBackDev::new();
        let sockets = SpinLock::new(SocketSet::new(vec![]));

        let server_socket = create_new_socket();
        let server_handle = sockets.lock().add(server_socket);
        let client_socket = create_new_socket();
        let client_handle = sockets.lock().add(client_socket);

        {
            let mut sockets_guard = sockets.lock();
            let server = sockets_guard.get_mut::<tcp::Socket>(server_handle);
            let local_endpoint =
                smoltcp::wire::IpEndpoint::new(smoltcp::wire::IpAddress::v4(127, 0, 0, 1), 80);
            server.listen(local_endpoint).unwrap();
            drop(sockets_guard);
        }

        {
            let mut sockets_guard = sockets.lock();
            let client = sockets_guard.get_mut::<tcp::Socket>(client_handle);
            let temp_port = 999;
            let remote_endpoint =
                smoltcp::wire::IpEndpoint::new(smoltcp::wire::IpAddress::v4(127, 0, 0, 1), 80);
            client
                .connect(
                    loopback.interface.lock().context(),
                    remote_endpoint,
                    temp_port,
                )
                .unwrap();
            drop(sockets_guard);
        }

        loop {
            let mut iface = loopback.interface.lock();
            let mut sockets_guard = sockets.lock();
            let timestamp = Instant::from_millis(get_time_ms() as i64);
            iface.poll(
                timestamp,
                loopback.dev.lock().deref_mut(),
                &mut sockets_guard,
            );
            let server = sockets_guard.get::<tcp::Socket>(server_handle);
            let client = sockets_guard.get::<tcp::Socket>(client_handle);
            debug!("server state: {:?}", server.state());
            debug!("client state: {:?}", client.state());
        }
    }
}
