//! Network Layer
use alloc::{boxed::Box, vec::Vec};
use core::task::Waker;

use async_trait::async_trait;
use smoltcp::{iface::SocketHandle, socket::tcp, wire::IpEndpoint};

use super::{
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
    net::{handle::HandleItem, port_manager, HANDLE_MAP},
    sched::utils::yield_now,
    syscall::SysResult,
    utils::crossover::intermit,
};

#[derive(PartialEq, Debug)]
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

        debug!("[Tcp] new socket: {}", new_socket_handle);

        let mut handle_map_guard = HANDLE_MAP.write();
        let item = HandleItem::new();
        handle_map_guard.insert(new_socket_handle, item);

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
            debug!(
                "[Tcp::do_listen {}] socket is already listening",
                self.handles[0]
            );
            return Ok(());
        }
        let local_endpoint = self.local_endpoint.ok_or(Errno::EINVAL)?;
        if local_endpoint.addr.is_unspecified() {
            debug!(
                "[Tcp::do_listen {}] local endpoint: {}:{} is unspecified",
                self.handles[0], local_endpoint.addr, local_endpoint.port
            );
            // let end_point = IpEndpoint::new(IpAddress::v4(127, 0, 0, 1),
            // local_endpoint.port); debug!("[Tcp::do_listen] listening addr:
            // {}", local_endpoint.addr); debug!("[Tcp::do_listen] listening
            // port: {}", local_endpoint.port);
            debug!(
                "[Tcp::do_listen {}] listening: {:?}",
                self.handles[0], local_endpoint.port
            );
            socket
                .listen(local_endpoint.port)
                .map_err(|_| Errno::EINVAL)?;
        } else {
            debug!(
                "[Tcp::do_listen {}] listening: {:?}",
                self.handles[0], local_endpoint
            );
            socket.listen(local_endpoint).map_err(|_| Errno::EINVAL)?;
        }
        self.state = TcpState::Listen;
        assert!(socket.is_listening());
        debug!(
            "[Tcp::do_listen {}] socket state: {:?}",
            self.handles[0],
            socket.state()
        );
        Ok(())
    }

    pub fn poll(&self, req: &PollEvent, waker: Waker) -> PollEvent {
        poll_ifaces();
        let mut res = PollEvent::empty();
        let mut sockets = SOCKET_SET.lock();
        for (handle, s) in sockets.iter() {
            match s {
                smoltcp::socket::Socket::Tcp(tcp) => {
                    debug!(
                        "[Tcp {}] poll: socket handle {}, state {:?}",
                        self.handles[0],
                        handle,
                        tcp.state()
                    );
                }
                _ => {}
            }
        }
        let socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);
        if req.contains(PollEvent::POLLIN) {
            debug!("[Tcp {}] poll: req has POLLIN", self.handles[0]);
            if socket.can_recv() {
                debug!("[Tcp {}] poll: POLLIN is ready 1", self.handles[0]);
                res |= PollEvent::POLLIN | PollEvent::POLLRDNORM;
            } else {
                match socket.state() {
                    tcp::State::CloseWait
                    | tcp::State::FinWait2
                    | tcp::State::TimeWait
                    | tcp::State::SynReceived => {
                        debug!("[Tcp {}] poll: POLLIN is ready 2", self.handles[0]);
                        res |= PollEvent::POLLIN | PollEvent::POLLRDNORM;
                    }
                    tcp::State::Established => {
                        if self.state == TcpState::Listen {
                            debug!("[Tcp {}] poll: POLLIN is ready 3", self.handles[0]);
                            res |= PollEvent::POLLIN | PollEvent::POLLRDNORM;
                        } else {
                            debug!(
                                "[Tcp {}] self state: {:?}, Established poll: register recv_waker",
                                self.handles[0], self.state
                            );
                            socket.register_recv_waker(&waker);
                        }
                    }
                    state => {
                        debug!(
                            "[Tcp {}] {:?} poll: register recv_waker",
                            self.handles[0], state
                        );
                        socket.register_recv_waker(&waker);
                    }
                }
            }
        }

        if req.contains(PollEvent::POLLOUT) {
            debug!("[Tcp {}] poll: req has POLLOUT", self.handles[0]);
            if socket.can_send() {
                debug!("[Tcp {}] poll: POLLOUT is ready", self.handles[0]);
                res |= PollEvent::POLLOUT | PollEvent::POLLWRNORM;
            } else {
                debug!("[Tcp {}] poll: register recv_waker", self.handles[0]);
                socket.register_send_waker(&waker);
            }
        }

        res
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
        debug!("[Tcp {}] read", self.handles[0]);
        if HANDLE_MAP
            .read()
            .get(&self.handles[0])
            .unwrap()
            .get_shutdown_type()
            .contains(ShutdownType::RCV_SHUTDOWN)
        {
            warn!("[Tcp {}] read: socket is closed", self.handles[0]);
            return (Err(Errno::ENOTCONN), None);
        }
        loop {
            poll_ifaces();
            let mut sockets = SOCKET_SET.lock();
            let socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);

            // if socket is closed, return error
            if !socket.is_active() {
                debug!(
                    "[Tcp {}] Socket Read Error, socket is closed",
                    self.handles[0]
                );
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
                            debug!(
                                "[Tcp {}] read receive: {:?}",
                                self.handles[0],
                                alloc::string::String::from_utf8_lossy(buf)
                            );
                            return (Ok(size), Some(remote_endpoint.unwrap()));
                        } else {
                            debug!("[Tcp {}] read receive: 0, yield!", self.handles[0]);
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
                        debug!("[Tcp {}] read receive: Finished", self.handles[0]);
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
        debug!("[Tcp {}] write: {}", self.handles[0], self.handles[0]);
        if HANDLE_MAP
            .read()
            .get(&self.handles[0])
            .unwrap()
            .get_shutdown_type()
            .contains(ShutdownType::RCV_SHUTDOWN)
        {
            warn!("[Tcp {}] write: socket is closed", self.handles[0]);
            return Err(Errno::ENOTCONN);
        }
        let mut sockets = SOCKET_SET.lock();
        let socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);

        if socket.is_open() {
            if socket.can_send() {
                match socket.send_slice(buf) {
                    Ok(size) => {
                        drop(sockets);
                        poll_ifaces();
                        debug!(
                            "[Tcp {}] write send: {:?}",
                            self.handles[0],
                            alloc::string::String::from_utf8_lossy(buf)
                        );
                        yield_now().await; // fixme: yield to let other task to run!
                        Ok(size)
                    }
                    Err(e) => {
                        error!("[Tcp {}] Socket Write Error {e:?}", self.handles[0]);
                        Err(Errno::ENOBUFS)
                    }
                }
            } else {
                error!(
                    "[Tcp {}] write: No buffer space available.",
                    self.handles[0]
                );
                Err(Errno::ENOBUFS)
            }
        } else {
            error!("[Tcp {}] write: socket is closed", self.handles[0]);
            Err(Errno::ENOTCONN)
        }
    }

    fn bind(&mut self, local: IpEndpoint, fd: usize) -> SysResult<()> {
        debug!("[Tcp {}] bind to {:?}", self.handles[0], local);
        let mut port_manager = TCP_PORT_MANAGER.lock();
        port_manager.bind_port_with_fd(local.port, fd)?;
        self.local_endpoint = Some(local);
        Ok(())
    }

    /// `backlog` is the maximum length to which the queue of pending
    /// connections
    ///
    /// return: whether the operation is successful
    fn listen(&mut self, _backlog: usize) -> SysResult<()> {
        // const MAX_BACKLOG: usize = 10;
        if self.state == TcpState::Listen {
            debug!(
                "[Tcp {}] listen: socket is already listening",
                self.handles[0]
            );
            return Ok(());
        }

        // let mut backlog = backlog;
        // if backlog > MAX_BACKLOG {
        //     warn!("[Tcp {}] listen backlog is too large, set to {}", MAX_BACKLOG);
        //     backlog = MAX_BACKLOG;
        // }

        let handlen = self.handles.len();
        // let backlog = handlen.max(backlog);
        let mut sockets = SOCKET_SET.lock();
        // let mut handle_map = HANDLE_MAP.write();

        // self.handles.extend((handlen..backlog).map(|_| {
        //     let new_socket = Self::new_socket();
        //     let handle = sockets.add(new_socket);
        //     let item = HandleItem::new(); // todo:
        //     handle_map.insert(handle, item);
        //     handle
        // }));

        (0..handlen).for_each(|i| {
            let handle = self.handles[i];
            debug!(
                "[Tcp {}] new socket {} is begin to listen",
                self.handles[0], handle
            );
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
        debug!("[Tcp {}] begin connect to {:?}", self.handles[0], remote);
        assert!(
            !remote.addr.is_unspecified(),
            "[Tcp {}] remote endpoint is unspecified",
            self.handles[0]
        );
        assert_ne!(remote.port, 0, "[Tcp {}] remote port is 0", self.handles[0]);

        let mut sockets = SOCKET_SET.lock();
        let local_socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);

        let mut port_manager = TCP_PORT_MANAGER.lock();
        let temp_port = port_manager.get_ephemeral_port()?;
        port_manager.bind_port(temp_port)?;
        drop(port_manager);

        let driver_write_guard = NET_DEVICES.write();
        let iface = driver_write_guard.get(&0).unwrap().clone(); // now we only have one net device
        drop(driver_write_guard);
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
                    intermit(|| debug!("[Tcp {}] connect loop: Synsent", self.handles[0]));
                    drop(sockets);
                    yield_now().await;
                }
                tcp::State::Established => {
                    debug!("[Tcp {}] connect loop: Established", self.handles[0]);
                    return Ok(());
                }
                _ => {
                    error!("[Tcp {}] connect loop: InvalidState", self.handles[0]);
                    return Err(Errno::ECONNREFUSED);
                }
            }
        }
    }

    /// It is used to accept a new incoming connection.
    async fn accept(&mut self) -> SysResult<(TcpSocket, IpEndpoint)> {
        debug!("[Tcp {}] accept", self.handles[0]);
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
                // replace the handle vector
                let new_socket = Self::new_socket();
                let new_socket_handle = sockets.add(new_socket);
                let old_socket_handle =
                    core::mem::replace(&mut self.handles[handle_index], new_socket_handle);
                let ret_old_socket = TcpSocket::from_handle(
                    old_socket_handle,
                    self.meta.options,
                    self.local_endpoint,
                );

                let old_socket = sockets.get::<tcp::Socket>(old_socket_handle);
                let remote_endpoint = old_socket.remote_endpoint().ok_or(Errno::ENOTCONN)?;

                // update HANDLE_MAP
                let mut handle_map_guard = HANDLE_MAP.write();
                let mut old = handle_map_guard.remove(&old_socket_handle).unwrap();
                old.set_shutdown_type(ShutdownType::empty());

                let new_item = HandleItem::new();
                handle_map_guard.insert(old_socket_handle, new_item);
                handle_map_guard.insert(new_socket_handle, old);

                drop(handle_map_guard);

                // relisten the new socket
                let new_socket = sockets.get_mut::<tcp::Socket>(new_socket_handle);
                if !new_socket.is_listening() {
                    self.do_listen(new_socket)?;
                }

                return Ok((ret_old_socket, remote_endpoint));
            }

            yield_now().await;
        }
    }

    /// return: whether the operation is successful
    fn shutdown(&mut self, operation: ShutdownType) -> SysResult<()> {
        let mut sockets = SOCKET_SET.lock();
        let local_socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);
        if operation.contains(ShutdownType::RCV_SHUTDOWN) {
            info!("[TcpSocket::shutdown {}] socket close", self.handles[0]);
            local_socket.close();
        } else {
            info!("[TcpSocket::shutdown {}] socket abort", self.handles[0]);
            local_socket.abort();
        }
        let mut handle_map_guard = HANDLE_MAP.write();
        handle_map_guard
            .get_mut(self.handle())
            .unwrap()
            .set_shutdown_type(operation);
        Ok(())
    }

    fn local_endpoint(&self) -> Option<IpEndpoint> {
        if self.local_endpoint.is_none() {
            let mut sockets = SOCKET_SET.lock();
            let local_socket = sockets.get_mut::<tcp::Socket>(self.handles[0]);
            local_socket.local_endpoint()
        } else {
            self.local_endpoint
        }
    }

    fn peer_endpoint(&self) -> Option<IpEndpoint> {
        let sockets = SOCKET_SET.lock();
        let local_socket = sockets.get::<tcp::Socket>(self.handles[0]);
        local_socket.remote_endpoint()
    }

    fn meta(&self) -> &SocketMeta {
        &self.meta
    }
}

impl Drop for TcpSocket {
    fn drop(&mut self) {
        warn!(
            "[Tcp {}] drop socket, local: {:?}",
            self.handles[0], self.local_endpoint
        );

        if let Some(local) = self.local_endpoint {
            let mut port_manager = TCP_PORT_MANAGER.lock();
            port_manager.unbind_port(local.port);
            drop(port_manager);
        }

        poll_ifaces();
        let mut sockets = SOCKET_SET.lock();
        let handle = self.handles[0];
        let socket = sockets.get_mut::<tcp::Socket>(handle);
        if socket.is_open() {
            socket.close();
            warn!("[Tcp {}] socket is closed", handle);
        }
        warn!(
            "[Tcp {}] after state is {:?}",
            self.handles[0],
            socket.state()
        );
        sockets.remove(handle);
        drop(sockets);
        poll_ifaces();

        // todo: handle map
    }
}
