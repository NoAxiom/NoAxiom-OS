use alloc::boxed::Box;

use async_trait::async_trait;
use downcast_rs::{impl_downcast, DowncastSync};
use smoltcp::{socket, wire::IpEndpoint};

use super::{tcpsocket::TcpSocket, NET_DEVICES, SOCKET_SET};
use crate::{
    include::net::{ShutdownType, SocketOptions, SocketType},
    syscall::SysResult,
};

#[derive(Debug, Clone)]
pub struct SocketMeta {
    /// socket的类型
    pub socket_type: SocketType,
    /// 接收缓冲区的大小
    pub rx_buf_size: usize,
    /// 发送缓冲区的大小
    pub tx_buf_size: usize,
    /// 元数据的缓冲区的大小
    pub metadata_buf_size: usize,
    /// socket的选项
    pub options: SocketOptions,
}

impl SocketMeta {
    pub fn new(
        socket_type: SocketType,
        rx_buf_size: usize,
        tx_buf_size: usize,
        metadata_buf_size: usize,
        options: SocketOptions,
    ) -> Self {
        Self {
            socket_type,
            rx_buf_size,
            tx_buf_size,
            metadata_buf_size,
            options,
        }
    }
}

/// TCP/UDP or other socket should implement this trait
#[async_trait]
// pub trait Socket: File {
pub trait Socket: Send + Sync + DowncastSync {
    /// Read data from the socket.
    ///
    /// `buf` is the buffer to store the read data
    ///
    /// return:
    /// - Success: (Returns the length of the data read, the endpoint
    /// from which data was read).
    /// - Failure: Error code
    async fn read(&self, buf: &mut [u8]) -> (SysResult<usize>, Option<IpEndpoint>);

    /// Write data to the socket.
    ///
    /// `buf` is the data to be written  
    /// `to` is the destination endpoint. If None, the written data will be
    /// discarded.
    ///
    /// return: the length of the data written
    async fn write(&self, buf: &[u8], to: Option<IpEndpoint>) -> SysResult<usize>;

    /// The bind() function is used to associate a socket with a particular IP
    /// address and port number on the local machine.
    ///
    /// return: whether the operation is successful
    fn bind(&mut self, local: IpEndpoint, fd: usize) -> SysResult<()>;

    /// `backlog` is the maximum length to which the queue of pending
    /// connections
    ///
    /// return: whether the operation is successful
    fn listen(&mut self, backlog: usize) -> SysResult<()>;

    /// It is used to establish a connection to a remote server.
    /// When a socket is connected to a remote server,
    /// the operating system will establish a network connection with the server
    /// and allow data to be sent and received between the local socket and the
    /// remote server.
    ///
    /// return: whether the operation is successful
    async fn connect(&mut self, remote: IpEndpoint) -> SysResult<()>;

    /// It is used to accept a new incoming connection.
    ///
    /// only used for TCP
    async fn accept(&mut self) -> SysResult<(TcpSocket, IpEndpoint)>;

    /// It is used to send data to a connected socket.
    ///
    /// return: whether the operation is successful
    fn shutdown(&mut self, operation: ShutdownType) -> SysResult<()>;

    /// Get the socket's metadata
    fn meta(&self) -> &SocketMeta;

    /// It is used to get the local endpoint of the socket.
    fn local_endpoint(&self) -> Option<IpEndpoint>;

    /// It is used to get the remote endpoint of the socket.
    fn peer_endpoint(&self) -> Option<IpEndpoint>;

    fn setsockopt(&self, _level: usize, _optname: usize, _optval: &[u8]) -> SysResult<()> {
        // warn!("setsockopt is not implemented");
        Ok(())
    }
}

impl_downcast!(sync Socket);

pub fn poll_ifaces() {
    let devices = NET_DEVICES.read();
    let mut sockets = SOCKET_SET.lock();
    for (_, iface) in devices.iter() {
        iface.poll(&mut sockets).ok();
    }
    // for (handle, socket) in sockets.iter_mut() {
    //     match socket {
    //         Tcp(socket) => {
    //             debug!(
    //                 "[poll_ifaces] poll Tcp socket {}, state: {:?}",
    //                 handle,
    //                 socket.state()
    //             );
    //         }
    //         Udp(_) => {
    //             debug!("[poll_ifaces] poll Udp socket {}", handle);
    //         }
    //         _ => unreachable!(),
    //     }
    // }
}
