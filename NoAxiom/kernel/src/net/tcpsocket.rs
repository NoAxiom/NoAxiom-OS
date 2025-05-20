//! Network Layer
use alloc::{boxed::Box, vec::Vec};

use async_trait::async_trait;
use smoltcp::{
    iface::SocketHandle,
    socket::tcp,
    wire::{IpAddress, IpEndpoint},
};

use super::{
    poll::SocketPollMethod,
    socket::{poll_ifaces, Socket, SocketMeta},
    NET_DEVICES, SOCKET_SET, TCP_PORT_MANAGER,
};
use crate::{
    constant::net::TCP_CONSTANTS,
    include::{
        io::PollEvent,
        net::{ShutdownType, SocketOptions, SocketType},
        result::Errno,
    },
    net::HANDLE_MAP,
    sched::utils::yield_now,
    syscall::SysResult,
};

#[derive(PartialEq)]
pub enum TcpState {
    Closed,
    Listen,
    // Established,
}

/// **TCP Socket** struct in kernel
///
/// this struct is under the protection of a big lock
pub struct TcpSocket {
    meta: SocketMeta,
    state: TcpState,
    /// for different end, the meaning of handles is different
    /// server: the handles is the listen socket handle
    /// client: the FIRST handle is the connect socket handle
    handles: Vec<SocketHandle>,
    local_endpoint: Option<IpEndpoint>,
}

impl TcpSocket {
    /// Create a new TcpSocket with `options`, handles contains a tcp socket
    pub fn new(options: SocketOptions) -> Self {
        let new_socket = Self::new_socket();
        let new_socket_handle = SOCKET_SET.insert(new_socket);

        let meta = SocketMeta::new(
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

    fn handle(&self) -> &SocketHandle {
        self.handles.first().unwrap()
    }

    fn from_handle(
        handle: SocketHandle,
        options: SocketOptions,
        local_endpoint: Option<IpEndpoint>,
    ) -> Self {
        let meta = SocketMeta::new(
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
            debug!("[Tcp::do_listen] socket is already listening");
            return Ok(());
        }
        let local_endpoint = self.local_endpoint.ok_or(Errno::EINVAL)?;
        if local_endpoint.addr.is_unspecified() {
            debug!(
                "[Tcp::do_listen] local endpoint: {}:{} is unspecified",
                local_endpoint.addr, local_endpoint.port
            );
            // let end_point = IpEndpoint::new(IpAddress::v4(127, 0, 0, 1),
            // local_endpoint.port);
            debug!("[Tcp::do_listen] listening addr: {}", local_endpoint.addr);
            debug!("[Tcp::do_listen] listening port: {}", local_endpoint.port);
            socket
                .listen(local_endpoint.port)
                .map_err(|_| Errno::EINVAL)?;
        } else {
            debug!("[Tcp::do_listen] listening: {:?}", local_endpoint);
            socket.listen(local_endpoint).map_err(|_| Errno::EINVAL)?;
        }
        self.state = TcpState::Listen;
        assert!(socket.is_listening());
        debug!("[Tcp::do_listen] socket state: {:?}", socket.state());
        Ok(())
    }

    pub fn poll(&self) -> PollEvent {
        if self.state == TcpState::Listen {
            let sockets = SOCKET_SET.lock();
            let can_accept = self.handles.iter().any(|handle| {
                let socket = sockets.get::<tcp::Socket>(*handle);
                socket.is_active()
            });
            drop(sockets);

            if can_accept {
                return PollEvent::POLLIN | PollEvent::POLLRDNORM;
            } else {
                return PollEvent::empty();
            }
        }

        assert!(self.handles.len() == 1);

        let sockets = SOCKET_SET.lock();
        let socket = sockets.get::<tcp::Socket>(self.handles[0]);
        let handle_map_guard = HANDLE_MAP.read();
        let shutdown_type = handle_map_guard
            .get(self.handle())
            .unwrap()
            .get_shutdown_type();
        return SocketPollMethod::tcp_poll(socket, shutdown_type);
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
                        let mut handle_map_guard = HANDLE_MAP.write();
                        handle_map_guard
                            .get_mut(self.handle())
                            .unwrap()
                            .set_shutdown_type(ShutdownType::RCV_SHUTDOWN);
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
        debug!("[Tcp] bind to {:?}", local);
        let mut port_manager = TCP_PORT_MANAGER.lock();
        port_manager.bind_port(local.port)?;
        self.local_endpoint = Some(local);
        Ok(())
    }

    /// `backlog` is the maximum length to which the queue of pending
    /// connections
    ///
    /// return: whether the operation is successful
    fn listen(&mut self, backlog: usize) -> SysResult<()> {
        const MAX_BACKLOG: usize = 10;
        if self.state == TcpState::Listen {
            return Ok(());
        }

        let mut backlog = backlog;
        if backlog > MAX_BACKLOG {
            warn!("[Tcp] listen backlog is too large, set to {}", MAX_BACKLOG);
            backlog = MAX_BACKLOG;
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
            debug!("[Tcp] new socket {} is begin to listen", handle);
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
        debug!("[Tcp] {} begin connect to {:?}", self.handles[0], remote);
        if remote.addr.is_unspecified() {
            warn!("[Tcp] remote endpoint is unspecified");
        }
        let mut sockets = SOCKET_SET.lock();
        let local_socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);

        let mut port_manager = TCP_PORT_MANAGER.lock();
        let temp_port = port_manager.get_ephemeral_port()?;
        // check whether the port is used, if not, bind it
        port_manager.bind_port(temp_port)?;
        drop(port_manager);

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
        drop(driver_write_guard);
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
                    debug!("[Tcp] connect loop: Synsent");
                    drop(sockets);
                    yield_now().await;
                }
                tcp::State::Established => {
                    debug!("[Tcp] connect loop: Established");
                    return Ok(());
                }
                _ => {
                    error!("[Tcp] connect loop: InvalidState");
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
        let mut handle_map_guard = HANDLE_MAP.write();
        handle_map_guard
            .get_mut(self.handle())
            .unwrap()
            .set_shutdown_type(operation);
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

    fn meta(&self) -> &SocketMeta {
        &self.meta
    }
}
