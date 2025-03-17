//! Network Layer
use alloc::boxed::Box;
use core::task::Waker;

use async_trait::async_trait;
use atomic_enum::atomic_enum;
use smoltcp::{iface::SocketHandle, socket::tcp, wire::IpEndpoint};

use super::{socket::Socket, PORT_MANAGER, SOCKET_SET};
use crate::{
    constant::net::{DEFAULT_RX_BUF_SIZE, DEFAULT_TX_BUF_SIZE},
    syscall::SysResult,
};

#[atomic_enum]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

pub struct TcpSocket {
    state: TcpState,
    handle: SocketHandle,
    local_endpoint: IpEndpoint,
    remote_endpoint: IpEndpoint,
}

impl TcpSocket {
    pub fn new(
        inner: SocketHandle,
        local_endpoint: IpEndpoint,
        remote_endpoint: IpEndpoint,
    ) -> Self {
        let new_socket = Self::new_socket();
        SOCKET_SET.insert(new_socket);
        Self {
            state: TcpState::Closed,
            handle: inner,
            local_endpoint,
            remote_endpoint,
        }
    }
    pub fn new_socket() -> tcp::Socket<'static> {
        let rx_buffer = tcp::SocketBuffer::new(vec![0; DEFAULT_RX_BUF_SIZE]);
        let tx_buffer = tcp::SocketBuffer::new(vec![0; DEFAULT_TX_BUF_SIZE]);
        tcp::Socket::new(rx_buffer, tx_buffer)
    }
}

#[async_trait]
impl Socket for TcpSocket {
    fn bind(&mut self, local: IpEndpoint) -> SysResult<usize> {
        self.local_endpoint = local;
        let waker = Waker::from(0);
        PORT_MANAGER.bind_port(local.into(), waker)?;
        Ok(0)
    }

    /// `backlog` is the maximum length to which the queue of pending
    /// connections
    ///
    /// return: whether the operation is successful
    fn listen(&mut self, backlog: usize) -> SysResult<usize>;

    /// It is used to establish a connection to a remote server.
    /// When a socket is connected to a remote server,
    /// the operating system will establish a network connection with the server
    /// and allow data to be sent and received between the local socket and the
    /// remote server.
    ///
    /// return: whether the operation is successful
    async fn connect(&mut self, remote: IpEndpoint) -> SysResult<usize>;

    /// It is used to accept a new incoming connection.
    async fn accept(&mut self) -> SysResult<Arc<dyn Socket>>;

    /// It is used to send data to a connected socket.
    ///
    /// return: whether the operation is successful
    fn shutdown(&mut self, operation: ShutdownType) -> SysResult<usize>;

    fn end_point(&self) -> IpEndpoint;
}
