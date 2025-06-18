use alloc::{collections::btree_map::BTreeMap, sync::Arc};

use driver::devices::impls::net::NetWorkDev;
use ksync::mutex::{RwLock, SpinLock};
use port_manager::PortManager;
use socket_set::SocketSet;

// mod handle;
// mod poll;
mod port_manager;
mod socket;
mod socket_set;
pub mod socketfile;
mod tcpsocket;
mod udpsocket;

lazy_static::lazy_static! {
    pub static ref SOCKET_SET: SocketSet = SocketSet::new();
    // pub static ref HANDLE_MAP: RwLock<BTreeMap<SocketHandle, HandleItem>> = RwLock::new(BTreeMap::new());
    pub static ref TCP_PORT_MANAGER: Arc<SpinLock<PortManager>> = Arc::new(SpinLock::new(PortManager::new()));
    pub static ref UDP_PORT_MANAGER: Arc<SpinLock<PortManager>> = Arc::new(SpinLock::new(PortManager::new()));
    pub static ref NET_DEVICES: RwLock<BTreeMap<usize, Arc<&'static dyn NetWorkDev>>> = {
        let net_devices = RwLock::new(BTreeMap::new());
        net_devices.write().insert(0, driver::get_net_dev());
        net_devices
    };
}

pub fn get_old_socket_fd(port: u16) -> usize {
    let port_manager = UDP_PORT_MANAGER.lock();
    if let Some(port_item) = port_manager.inner.get(&port) {
        port_item.fd
    } else {
        drop(port_manager);
        let port_manager = TCP_PORT_MANAGER.lock();
        error!("[tcp_port_manager] Port {port} is already listened");
        let item = port_manager
            .inner
            .get(&port)
            .expect("Port {port} is not listened");
        item.fd
    }
}

#[allow(dead_code)]
pub mod test {
    use core::ops::DerefMut;

    use driver::devices::impls::net::loopback::LoopBackDev;
    use ksync::mutex::SpinLock;
    use smoltcp::{
        iface::SocketSet,
        socket::{tcp, udp},
        time::Instant,
    };

    use crate::{constant::net::UDP_CONSTANTS, time::gettime::get_time_ms};

    /// 元数据的缓冲区的大小
    pub const DEFAULT_METADATA_BUF_SIZE: usize = 1024;
    /// 默认的接收缓冲区的大小 receive
    pub const DEFAULT_RX_BUF_SIZE: usize = 512 * 1024;
    /// 默认的发送缓冲区的大小 transmiss
    pub const DEFAULT_TX_BUF_SIZE: usize = 512 * 1024;

    pub fn net_test() {
        // net_tcp_test();
        net_udp_test();
    }

    fn create_new_tcp_socket() -> tcp::Socket<'static> {
        // 初始化tcp的buffer
        let rx_buffer = tcp::SocketBuffer::new(vec![0; DEFAULT_RX_BUF_SIZE]);
        let tx_buffer = tcp::SocketBuffer::new(vec![0; DEFAULT_TX_BUF_SIZE]);
        tcp::Socket::new(rx_buffer, tx_buffer)
    }

    fn create_new_udp_socket() -> udp::Socket<'static> {
        let rx_buffer = udp::PacketBuffer::new(
            vec![udp::PacketMetadata::EMPTY; UDP_CONSTANTS.default_metadata_buf_size],
            vec![0; UDP_CONSTANTS.default_rx_buf_size],
        );
        let tx_buffer = udp::PacketBuffer::new(
            vec![udp::PacketMetadata::EMPTY; UDP_CONSTANTS.default_metadata_buf_size],
            vec![0; UDP_CONSTANTS.default_tx_buf_size],
        );
        udp::Socket::new(rx_buffer, tx_buffer)
    }

    pub fn net_tcp_test() {
        let loopback = LoopBackDev::new();
        let sockets = SpinLock::new(SocketSet::new(vec![]));

        let server_socket = create_new_tcp_socket();
        let server_handle = sockets.lock().add(server_socket);
        let server_socket_2 = create_new_tcp_socket();
        let server_handle_2 = sockets.lock().add(server_socket_2);
        let client_socket = create_new_tcp_socket();
        let client_handle = sockets.lock().add(client_socket);
        let client_socket_2 = create_new_tcp_socket();
        let client_handle_2 = sockets.lock().add(client_socket_2);

        {
            let mut sockets_guard = sockets.lock();
            let server = sockets_guard.get_mut::<tcp::Socket>(server_handle);
            server.listen(80).unwrap();
            drop(sockets_guard);
        }

        {
            let mut sockets_guard = sockets.lock();
            let server = sockets_guard.get_mut::<tcp::Socket>(server_handle_2);
            server.listen(80).unwrap();
            drop(sockets_guard);
        }

        {
            let mut sockets_guard = sockets.lock();
            let client = sockets_guard.get_mut::<tcp::Socket>(client_handle);
            let remote_endpoint =
                smoltcp::wire::IpEndpoint::new(smoltcp::wire::IpAddress::v4(127, 0, 0, 1), 80);
            client
                .connect(loopback.interface.lock().context(), remote_endpoint, 233)
                .unwrap();
            drop(sockets_guard);
        }

        {
            let mut sockets_guard = sockets.lock();
            let client = sockets_guard.get_mut::<tcp::Socket>(client_handle_2);
            let remote_endpoint =
                smoltcp::wire::IpEndpoint::new(smoltcp::wire::IpAddress::v4(127, 0, 0, 1), 80);
            client
                .connect(loopback.interface.lock().context(), remote_endpoint, 234)
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
            let server_2 = sockets_guard.get::<tcp::Socket>(server_handle_2);
            let client = sockets_guard.get::<tcp::Socket>(client_handle);
            let client_2 = sockets_guard.get::<tcp::Socket>(client_handle_2);
            debug!("server   state: {:?}", server.state());
            debug!("server_2 state: {:?}", server_2.state());
            debug!("client   state: {:?}", client.state());
            debug!("client_2 state: {:?}", client_2.state());
        }
    }

    pub fn net_udp_test() {
        let loopback = LoopBackDev::new();
        let sockets = SpinLock::new(SocketSet::new(vec![]));

        let server_socket = create_new_udp_socket();
        let server_handle = sockets.lock().add(server_socket);
        let server_socket_2 = create_new_udp_socket();
        let server_handle_2 = sockets.lock().add(server_socket_2);

        let client_socket = create_new_udp_socket();
        let client_handle = sockets.lock().add(client_socket);
        let client_socket_2 = create_new_udp_socket();
        let client_handle_2 = sockets.lock().add(client_socket_2);

        {
            let mut sockets_guard = sockets.lock();
            let server = sockets_guard.get_mut::<udp::Socket>(server_handle);
            server.bind(5001).unwrap();
            drop(sockets_guard);
        }

        {
            let mut sockets_guard = sockets.lock();
            let server = sockets_guard.get_mut::<udp::Socket>(server_handle_2);
            server.bind(5001).unwrap();
            drop(sockets_guard);
        }

        {
            let mut sockets_guard = sockets.lock();
            let client = sockets_guard.get_mut::<udp::Socket>(client_handle);
            client.bind(233).unwrap();
            drop(sockets_guard);
        }

        {
            let mut sockets_guard = sockets.lock();
            let client = sockets_guard.get_mut::<udp::Socket>(client_handle_2);
            client.bind(234).unwrap();
            drop(sockets_guard);
        }

        const MAX_SEND: usize = 100;
        let mut sent = MAX_SEND;

        loop {
            let mut iface = loopback.interface.lock();
            let mut sockets_guard = sockets.lock();
            let timestamp = Instant::from_millis(get_time_ms() as i64);
            iface.poll(
                timestamp,
                loopback.dev.lock().deref_mut(),
                &mut sockets_guard,
            );
            drop(sockets_guard);
            drop(iface);

            let mut sockets_guard = sockets.lock();
            let client = sockets_guard.get_mut::<udp::Socket>(client_handle);
            if client.can_send() && sent != 0 {
                sent -= 1;
                let data = b"Hello, UDP!";
                let remote_endpoint = smoltcp::wire::IpEndpoint::new(
                    smoltcp::wire::IpAddress::v4(127, 0, 0, 1),
                    5001,
                );
                client.send_slice(data, remote_endpoint).unwrap();
            }
            drop(sockets_guard);

            let mut sockets_guard = sockets.lock();
            let server = sockets_guard.get_mut::<udp::Socket>(server_handle);
            if server.can_recv() {
                let (data, remote_endpoint) = server.recv().unwrap();
                debug!(
                    "#{} Server received data: {:?} from {}",
                    MAX_SEND - sent,
                    data,
                    remote_endpoint
                );
            }
            drop(sockets_guard);

            let mut sockets_guard = sockets.lock();
            let server_2 = sockets_guard.get_mut::<udp::Socket>(server_handle_2);
            if server_2.can_recv() {
                let (data, remote_endpoint) = server_2.recv().unwrap();
                debug!(
                    "#{} Server_2 received data: {:?} from {}",
                    MAX_SEND - sent,
                    data,
                    remote_endpoint
                );
            }
            drop(sockets_guard);
        }
    }
}
