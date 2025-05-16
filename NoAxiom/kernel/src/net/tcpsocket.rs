//! Network Layer
use alloc::{boxed::Box, vec::Vec};

use async_trait::async_trait;
use smoltcp::{iface::SocketHandle, socket::tcp, wire::IpEndpoint};

use super::{
    socket::{poll_ifaces, Socket, SocketMetadata},
    NET_DEVICES, PORT_MANAGER, SOCKET_SET,
};
use crate::{
    constant::net::TCP_CONSTANTS,
    include::{
        net::{ShutdownType, SocketOptions, SocketType},
        result::Errno,
    },
    sched::utils::yield_now,
    syscall::SysResult,
};

#[derive(PartialEq)]
pub enum TcpState {
    Closed,
    Listen,
    // Established,
}

pub struct TcpSocket {
    meta: SocketMetadata,
    state: TcpState,
    /// for different end, the meaning of handles is different
    /// server: the handles is the listen socket handle
    /// client: the FIRST handle is the connect socket handle
    handles: Vec<SocketHandle>,
    local_endpoint: Option<IpEndpoint>,
    // todo: nonblock?
}

impl TcpSocket {
    /// Create a new TcpSocket with `options`, handles contains a tcp socket
    pub fn new(options: SocketOptions) -> Self {
        let new_socket = Self::new_socket();
        let new_socket_handle = SOCKET_SET.insert(new_socket);

        let meta = SocketMetadata::new(
            SocketType::Tcp,
            TCP_CONSTANTS.default_rx_buf_size,
            TCP_CONSTANTS.default_tx_buf_size,
            TCP_CONSTANTS.default_metadata_buf_size,
            options,
        );

        debug!("[Tcp] new socket: {:?}", new_socket_handle);
        Self {
            meta,
            state: TcpState::Closed,
            handles: vec![new_socket_handle],
            local_endpoint: None,
        }
    }

    fn from_handle(
        handle: SocketHandle,
        options: SocketOptions,
        local_endpoint: Option<IpEndpoint>,
    ) -> Self {
        let meta = SocketMetadata::new(
            SocketType::Tcp,
            TCP_CONSTANTS.default_rx_buf_size,
            TCP_CONSTANTS.default_tx_buf_size,
            TCP_CONSTANTS.default_metadata_buf_size,
            options,
        );

        Self {
            meta,
            state: TcpState::Closed,
            handles: vec![handle],
            local_endpoint,
        }
    }

    /// Create a new smoltcp's tcp::Socket
    pub fn new_socket() -> tcp::Socket<'static> {
        let rx_buffer = tcp::SocketBuffer::new(vec![0; TCP_CONSTANTS.default_rx_buf_size]);
        let tx_buffer = tcp::SocketBuffer::new(vec![0; TCP_CONSTANTS.default_tx_buf_size]);
        tcp::Socket::new(rx_buffer, tx_buffer)
    }

    fn do_listen(&mut self, socket: &mut tcp::Socket<'static>) -> SysResult<()> {
        if socket.is_listening() {
            return Ok(());
        }
        let local_endpoint = self.local_endpoint.ok_or(Errno::EINVAL)?;
        if local_endpoint.addr.is_unspecified() {
            socket
                .listen(local_endpoint.port)
                .map_err(|_| Errno::EINVAL)?;
        } else {
            socket.listen(local_endpoint).map_err(|_| Errno::EINVAL)?;
        }
        self.state = TcpState::Listen;
        Ok(())
    }
}

#[async_trait]
impl Socket for TcpSocket {
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
            let socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);

            // if socket is closed, return error
            if !socket.is_active() {
                // debug!("Tcp Socket Read Error, socket is closed");
                return (Err(Errno::ENOTCONN), None);
            }

            if socket.may_recv() {
                match socket.recv_slice(buf) {
                    Ok(size) => {
                        if size > 0 {
                            let remote_endpoint = socket.remote_endpoint();
                            if remote_endpoint.is_none() {
                                return (Err(Errno::ENOTCONN), None);
                            }
                            drop(sockets);
                            poll_ifaces();
                            return (Ok(size), Some(remote_endpoint.unwrap()));
                        }
                    }
                    Err(tcp::RecvError::InvalidState) => {
                        warn!("Tcp Socket Read Error, InvalidState");
                        return (Err(Errno::ENOTCONN), None);
                    }
                    Err(tcp::RecvError::Finished) => {
                        // remote write end is closed, we should close the read end
                        return (Err(Errno::ENOTCONN), None);
                    }
                }
            } else {
                return (Err(Errno::ENOTCONN), None);
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
    async fn write(&self, buf: &[u8], _to: Option<IpEndpoint>) -> SysResult<usize> {
        let mut sockets = SOCKET_SET.lock();
        let socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);

        if socket.is_open() {
            if socket.can_send() {
                match socket.send_slice(buf) {
                    Ok(size) => {
                        drop(sockets);
                        poll_ifaces();
                        Ok(size)
                    }
                    Err(e) => {
                        error!("Tcp Socket Write Error {e:?}");
                        Err(Errno::ENOBUFS)
                    }
                }
            } else {
                Err(Errno::ENOBUFS)
            }
        } else {
            Err(Errno::ENOTCONN)
        }
    }

    fn bind(&mut self, local: IpEndpoint) -> SysResult<()> {
        PORT_MANAGER.bind_port::<tcp::Socket<'static>>(local.port)?;
        self.local_endpoint = Some(local);
        Ok(())
    }

    /// `backlog` is the maximum length to which the queue of pending
    /// connections
    ///
    /// return: whether the operation is successful
    fn listen(&mut self, backlog: usize) -> SysResult<()> {
        if self.state == TcpState::Listen {
            return Ok(());
        }

        let handlen = self.handles.len();
        let backlog = handlen.max(backlog);
        let mut sockets = SOCKET_SET.lock();

        self.handles.extend((handlen..backlog).map(|_| {
            let new_socket = Self::new_socket();
            sockets.add(new_socket)
        }));

        (0..backlog).for_each(|i| {
            let handle = self.handles[i];
            let socket = sockets.get_mut::<tcp::Socket>(handle);
            self.do_listen(socket).unwrap();
        });

        Ok(())
    }

    /// It is used to establish a connection to a remote server.
    /// When a socket is connected to a remote server,
    /// the operating system will establish a network connection with the server
    /// and allow data to be sent and received between the local socket and the
    /// remote server.
    ///
    /// return: whether the operation is successful
    async fn connect(&mut self, remote: IpEndpoint) -> SysResult<()> {
        debug!("[Tcp] connect to {:?}", remote);
        let mut sockets = SOCKET_SET.lock();
        let local_socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);

        let temp_port = PORT_MANAGER.get_ephemeral_port()?;
        // check whether the port is used, if not, bind it
        PORT_MANAGER.bind_port::<tcp::Socket<'static>>(temp_port)?;

        let driver_write_guard = NET_DEVICES.write();
        let iface = driver_write_guard.get(&0).unwrap().clone(); // now we only have one net device
        let mut iface_inner = iface.inner_iface().lock();

        local_socket
            .connect(iface_inner.context(), remote, temp_port)
            .map_err(|e| match e {
                tcp::ConnectError::InvalidState => Errno::EISCONN,
                tcp::ConnectError::Unaddressable => Errno::EADDRNOTAVAIL,
            })?;

        drop(sockets);
        drop(iface_inner);
        loop {
            poll_ifaces();
            let mut sockets = SOCKET_SET.lock();
            let local_socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);

            /*
            Client                             Server

            CLOSED                             LISTEN
            |                                  |
            | -------- send SYN ----------->   |
            SynSent                            |
            |                                  |
            |      <----- recive SYN+ACK ----  SYN_RCVD
            |                                  |
            | -------- send ACK ----------->   |
            ESTABLISHED                        ESTABLISHED
            */
            match local_socket.state() {
                tcp::State::Closed => {
                    unreachable!()
                }
                tcp::State::SynSent => {
                    yield_now().await;
                }
                tcp::State::Established => {
                    return Ok(());
                }
                _ => {
                    return Err(Errno::ECONNREFUSED);
                }
            }
        }
    }

    /// It is used to accept a new incoming connection.
    async fn accept(&mut self) -> SysResult<(TcpSocket, IpEndpoint)> {
        if self.state != TcpState::Listen {
            return Err(Errno::EINVAL);
        }

        loop {
            poll_ifaces();
            let mut sockets = SOCKET_SET.lock();
            let chosen_handle_index = self.handles.iter().position(|handle| {
                let socket = sockets.get::<tcp::Socket>(*handle);
                socket.is_active()
            });

            if let Some(handle_index) = chosen_handle_index {
                let new_socket = Self::new_socket();
                let new_socket_handle = sockets.add(new_socket);
                let old_socket_handle = self.handles.remove(handle_index);
                let old_socket = TcpSocket::from_handle(
                    old_socket_handle,
                    self.meta.options,
                    self.local_endpoint,
                );

                self.handles.push(new_socket_handle);

                let remote_socket = sockets.get::<tcp::Socket>(old_socket_handle);
                let remote_endpoint = remote_socket.remote_endpoint().ok_or(Errno::ENOTCONN)?;

                return Ok((old_socket, remote_endpoint));
            }

            yield_now().await;
        }
    }

    /// It is used to send data to a connected socket.
    ///
    /// return: whether the operation is successful
    fn shutdown(&mut self, operation: ShutdownType) -> SysResult<()> {
        let mut sockets = SOCKET_SET.lock();
        let local_socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);
        if operation.contains(ShutdownType::RCV_SHUTDOWN) {
            info!("[TcpSocket::shutdown] socket close");
            local_socket.close();
        } else {
            info!("[TcpSocket::shutdown] socket abort");
            local_socket.abort();
        }
        Ok(())
    }

    fn end_point(&self) -> Option<IpEndpoint> {
        if self.local_endpoint.is_none() {
            let mut sockets = SOCKET_SET.lock();
            let local_socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);
            local_socket.local_endpoint()
        } else {
            self.local_endpoint
        }
    }
}
