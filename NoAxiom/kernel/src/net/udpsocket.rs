use alloc::{boxed::Box, vec};

use async_trait::async_trait;
use smoltcp::{
    iface::SocketHandle,
    socket::udp,
    wire::{IpAddress, IpEndpoint},
};

use super::{
    socket::{poll_ifaces, Socket, SocketMetadata},
    tcpsocket::TcpSocket,
    PORT_MANAGER, SOCKET_SET,
};
use crate::{
    constant::net::UDP_CONSTANTS,
    include::{
        net::{ShutdownType, SocketOptions, SocketType},
        result::Errno,
    },
    sched::utils::yield_now,
    syscall::SysResult,
};

pub struct UdpSocket {
    pub handle: SocketHandle,
    remote_endpoint: Option<IpEndpoint>, // for connect()
    meta: SocketMetadata,
}

impl UdpSocket {
    pub fn new(options: SocketOptions) -> Self {
        let new_socket = Self::new_socket();
        let new_socket_handle = SOCKET_SET.insert(new_socket);

        let meta = SocketMetadata::new(
            SocketType::Udp,
            UDP_CONSTANTS.default_rx_buf_size,
            UDP_CONSTANTS.default_tx_buf_size,
            UDP_CONSTANTS.default_metadata_buf_size,
            options,
        );
        debug!("[Udp] new socket: {:?}", new_socket_handle);
        Self {
            handle: new_socket_handle,
            remote_endpoint: None,
            meta,
        }
    }

    fn new_socket() -> udp::Socket<'static> {
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
}

#[async_trait]
impl Socket for UdpSocket {
    /// Read data from the socket.
    ///
    /// `buf` is the buffer to store the read data
    ///
    /// return:
    /// - Success: (Returns the length of the data read, the endpoint
    /// from which data was read).
    /// - Failure: Error code
    async fn read(&self, buf: &mut [u8]) -> (SysResult<usize>, Option<IpEndpoint>) {
        loop {
            poll_ifaces();
            let mut sockets = SOCKET_SET.lock();
            let socket = sockets.get_mut::<udp::Socket>(self.handle);

            if socket.can_recv() {
                if let Ok((size, metadata)) = socket.recv_slice(buf) {
                    drop(sockets);
                    poll_ifaces();
                    return (Ok(size), Some(metadata.endpoint));
                }
            } else {
                yield_now().await;
                // 如果socket没有连接，则忙等
                // return (Err(SystemError::ENOTCONN), Endpoint::Ip(None));
            }
            drop(sockets);
            yield_now().await;
        }
    }

    /// Write data to the socket, sync funciton.
    ///
    /// `buf` is the data to be written  
    /// `to` is the destination endpoint. If None, the written data will be
    /// discarded.
    ///
    /// return: the length of the data written
    async fn write(&self, buf: &[u8], to: Option<IpEndpoint>) -> SysResult<usize> {
        let remote_endpoint = {
            if let Some(ref endpoint) = to {
                endpoint
            } else if let Some(ref endpoint) = self.remote_endpoint {
                endpoint
            } else {
                return Err(Errno::ENOTCONN);
            }
        };

        let mut sockets = SOCKET_SET.lock();
        let socket = sockets.get_mut::<udp::Socket>(self.handle);

        if socket.can_send() {
            // debug!("udp write: can send");
            match socket.send_slice(buf, *remote_endpoint) {
                Ok(()) => {
                    // debug!("udp write: send ok");
                    drop(sockets);
                    poll_ifaces();
                    Ok(buf.len())
                }
                Err(_) => {
                    // debug!("udp write: send err");
                    Err(Errno::ENOBUFS)
                }
            }
        } else {
            Err(Errno::ENOBUFS)
        }
    }

    fn bind(&mut self, local: IpEndpoint) -> SysResult<()> {
        PORT_MANAGER.bind_port::<udp::Socket<'static>>(local.port)?;

        let mut sockets = SOCKET_SET.lock();
        let socket = sockets.get_mut::<udp::Socket>(self.handle);

        if local.addr.is_unspecified() {
            socket.bind(local.port)
        } else {
            socket.bind(local)
        }
        .map_err(|_| Errno::EINVAL)?;

        Ok(())
    }

    /// `backlog` is the maximum length to which the queue of pending
    /// connections
    ///
    /// return: whether the operation is successful
    fn listen(&mut self, _backlog: usize) -> SysResult<()> {
        // UDP is connectionless, so it does not support listen
        Err(Errno::ENOSYS)
    }

    /// It is used to establish a connection to a remote server.
    /// When a socket is connected to a remote server,
    /// the operating system will establish a network connection with the server
    /// and allow data to be sent and received between the local socket and the
    /// remote server.
    ///
    /// return: whether the operation is successful
    async fn connect(&mut self, remote: IpEndpoint) -> SysResult<()> {
        self.remote_endpoint = Some(remote);
        Ok(())
    }

    /// It is used to accept a new incoming connection.
    async fn accept(&mut self) -> SysResult<(TcpSocket, IpEndpoint)> {
        Err(Errno::ENOSYS)
    }

    /// It is used to send data to a connected socket.
    ///
    /// return: whether the operation is successful
    fn shutdown(&mut self, _operation: ShutdownType) -> SysResult<()> {
        Err(Errno::ENOSYS)
    }

    fn end_point(&self) -> Option<IpEndpoint> {
        let sockets = SOCKET_SET.lock();
        let socket = sockets.get::<udp::Socket>(self.handle);
        let listen_endpoint = socket.endpoint();

        if listen_endpoint.port == 0 {
            None
        } else {
            // support ipv4 only
            // TODO: support ipv6
            let endpoint = IpEndpoint::new(
                // if listen_endpoint.addr is None, it means "listen to all addresses"
                listen_endpoint.addr.unwrap_or(IpAddress::v4(0, 0, 0, 0)),
                listen_endpoint.port,
            );
            return Some(endpoint);
        }
    }
}
