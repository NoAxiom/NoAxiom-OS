use alloc::sync::Arc;

use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address};

use super::SyscallResult;
use crate::{
    fs::{pipe::PipeFile, vfs::basic::file::File},
    include::{
        fs::{FdFlags, FileFlags},
        net::{
            AddressFamily, MessageHeaderRaw, PosixSocketOption, PosixSocketType,
            PosixTcpSocketOptions, ShutdownType, SockAddr, SocketLevel, SOCK_CLOEXEC,
            SOCK_NONBLOCK,
        },
        result::Errno,
    },
    mm::user_ptr::UserPtr,
    net::socketfile::{Sock, SocketFile},
    signal::interruptable::interruptable,
    syscall::Syscall,
};

impl Syscall<'_> {
    pub fn sys_socket(
        &self,
        address_family: usize,
        socket_type: usize,
        protocol: usize,
    ) -> SyscallResult {
        info!(
            "[sys_socket] socket: address_family: {}, socket_type: {}, protocol: {}",
            address_family, socket_type, protocol
        );

        // Extract flags from socket_type
        let flags = socket_type & !0xf; // Get the upper bits (flags)
        let actual_socket_type = socket_type & 0xf; // Get the lower 4 bits (actual type)

        // Check for SOCK_CLOEXEC flag
        let has_cloexec = (flags & (crate::include::net::SOCK_CLOEXEC as usize)) != 0;
        let has_nonblock = (flags & (crate::include::net::SOCK_NONBLOCK as usize)) != 0;

        // todo: maybe should add socket inode
        let socket_file = SocketFile::new(
            AddressFamily::try_from(address_family as u16)?,
            PosixSocketType::try_from(actual_socket_type)?,
        )?;

        let mut fd_table = self.task.fd_table();
        let socket_fd = fd_table.alloc_fd()?;
        fd_table.set(socket_fd as usize, Arc::new(socket_file));

        // Set FD_CLOEXEC flag if SOCK_CLOEXEC was specified
        if has_cloexec {
            fd_table.set_fdflag(socket_fd as usize, &FdFlags::FD_CLOEXEC);
        }

        if has_nonblock {
            fd_table
                .get_socketfile(socket_fd)
                .unwrap()
                .meta()
                .set_flags(FileFlags::O_NONBLOCK);
        }

        debug!("[sys_socket] socket fd: {}", socket_fd);
        Ok(socket_fd as isize)
    }

    pub async fn sys_bind(&self, sockfd: usize, addr: usize, addr_len: usize) -> SyscallResult {
        info!("[sys_bind] sockfd: {}, addr: {}", sockfd, addr);
        let sock_addr = SockAddr::new(addr, addr_len)?;

        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        let res = socket.bind(sock_addr, sockfd);
        match res {
            // !fixme: now we SPECIALLY handle EADDRINUSE, to handle the case that multiple sockets
            // !use the same port, and we just assume at the same task
            Err(Errno::EADDRINUSE) => {
                warn!("[sys_bind] address already in use, so we copy from the old socket file");
                // get the old socket file
                let old_fd = crate::net::get_old_socket_fd(sock_addr.get_endpoint().port);

                // copy the old socket file to the current one, and the current one will be
                // dropped
                let mut fd_table = self.task.fd_table();
                fd_table.copyfrom(old_fd, sockfd)?;
                drop(fd_table);
                Ok(0)
            }
            _ => {
                debug!("[sys_bind] bind ok");
                Ok(0)
            }
        }
    }

    pub async fn sys_listen(&self, sockfd: usize, backlog: usize) -> SyscallResult {
        debug!("[sys_listen] sockfd: {}, backlog: {}", sockfd, backlog);
        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        socket.listen(backlog)?;

        Ok(0)
    }

    pub async fn sys_connect(&self, sockfd: usize, addr: usize, addr_len: usize) -> SyscallResult {
        debug!("[sys_connect] sockfd: {}, addr: {}", sockfd, addr);
        let sock_addr = SockAddr::new(addr, addr_len)?;

        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        socket.connect(sock_addr).await?;

        Ok(0)
    }

    pub async fn sys_accept(&self, sockfd: usize, addr: usize, _addrlen: usize) -> SyscallResult {
        info!("[sys_accept] sockfd: {}, addr: {}", sockfd, addr);
        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        let (new_tcp_socket, endpoint) =
            interruptable(self.task, socket.accept(), None, None).await??;

        // yield_now().await; // change to the process that just connect and yield

        let sockaddr = SockAddr::from_endpoint(endpoint);
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        debug!("[sys_accept] succeed endpoint: {:?}", endpoint);
        user_ptr.write(sockaddr).await?;

        let new_socket_file =
            SocketFile::new_from_socket(socket_file.clone(), Sock::Tcp(new_tcp_socket));
        let mut fd_table = self.task.fd_table();
        let new_fd = fd_table.alloc_fd()?;
        fd_table.set(new_fd as usize, Arc::new(new_socket_file));

        Ok(new_fd as isize)
    }

    pub async fn sys_accept4(
        &self,
        sockfd: usize,
        addr: usize,
        _addrlen: usize,
        flags: i32,
    ) -> SyscallResult {
        info!("[sys_accept4] sockfd: {}, addr: {}", sockfd, addr);
        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        let (new_tcp_socket, endpoint) =
            interruptable(self.task, socket.accept(), None, None).await??;

        // yield_now().await; // change to the process that just connect and yield

        let sockaddr = SockAddr::from_endpoint(endpoint);
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        debug!("[sys_accept] succeed endpoint: {:?}", endpoint);
        user_ptr.write(sockaddr).await?;

        let new_socket_file =
            SocketFile::new_from_socket(socket_file.clone(), Sock::Tcp(new_tcp_socket));
        let mut fd_table = self.task.fd_table();
        let new_fd = fd_table.alloc_fd()?;
        fd_table.set(new_fd as usize, Arc::new(new_socket_file));
        if flags & SOCK_CLOEXEC != 0 {
            fd_table.set_fdflag(new_fd, &FdFlags::FD_CLOEXEC);
        }
        if flags & SOCK_NONBLOCK != 0 {
            fd_table.set_nonblock(new_fd);
        }

        Ok(new_fd as isize)
    }

    pub async fn sys_getsockname(
        &self,
        sockfd: usize,
        addr: usize,
        _addrlen: usize,
    ) -> SyscallResult {
        info!("[sys_getsockname] sockfd: {}, addr: {}", sockfd, addr);
        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let socket = socket_file.socket().await;
        let local_endpoint = socket.local_endpoint().unwrap();
        drop(socket);

        let sockaddr = SockAddr::from_endpoint(local_endpoint);
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        debug!("[sys_getsockname] local endpoint: {:?}", local_endpoint);
        user_ptr.write(sockaddr).await?;

        Ok(0)
    }

    pub async fn sys_getpeername(
        &self,
        sockfd: usize,
        addr: usize,
        _addrlen: usize,
    ) -> SyscallResult {
        info!("[sys_getpeername] sockfd: {}, addr: {}", sockfd, addr);
        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let socket = socket_file.socket().await;
        let remote_endpoint = socket.peer_endpoint().ok_or(Errno::EINVAL)?;
        drop(socket);

        let sockaddr = SockAddr::from_endpoint(remote_endpoint);
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        debug!("[sys_getpeername] remote endpoint: {:?}", remote_endpoint);
        user_ptr.write(sockaddr).await?;

        Ok(0)
    }

    /// configure socket options
    pub async fn sys_setsockopt(
        &self,
        sockfd: usize,
        level: usize,
        optname: usize,
        optval_ptr: usize,
        optlen: usize,
    ) -> SyscallResult {
        // let level = PosixIpProtocol::from_repr(level as
        // u16).ok_or(Errno::ENOPROTOOPT)?;
        info!(
            "[sys_setsockopt] sockfd: {}, level: {:?}, optname: {}, optval_ptr: {}, optlen: {}",
            sockfd, level, optname, optval_ptr, optlen
        );

        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);
        let mut socket = socket_file.socket().await;

        let user_ptr = UserPtr::<u8>::new(optval_ptr);
        let buf_slice = user_ptr.as_slice_const_checked(optlen).await?;
        socket.setsockopt(level, optname, &buf_slice)?;
        Ok(0)
    }

    /// get socket options
    pub async fn sys_getsockopt(
        &self,
        sockfd: usize,
        level: usize,
        optname: usize,
        optval_ptr: usize,
        optlen: usize,
    ) -> SyscallResult {
        info!(
            "[sys_getsockopt] sockfd: {:?}, level: {:?}, optname: {:?}, optval_ptr: {:?}, optlen: {:?}",
            sockfd, level, optname, optval_ptr, optlen
        );

        let optvalptr = UserPtr::<u32>::new(optval_ptr);
        let optval = optvalptr.get_ref_mut().await?.ok_or(Errno::EFAULT)?;
        let optlenptr = UserPtr::<u32>::new(optlen);
        let optlen = optlenptr.get_ref_mut().await?.ok_or(Errno::EFAULT)?;

        match SocketLevel::try_from(level)? {
            SocketLevel::SOL_SOCKET => {
                const SEND_BUFFER_SIZE: usize = 64 * 1024;
                const RECV_BUFFER_SIZE: usize = 64 * 1024;
                match PosixSocketOption::from_repr(optname as i32) {
                    Some(opt) => match opt {
                        PosixSocketOption::SO_RCVBUF => {
                            *optval = RECV_BUFFER_SIZE as u32;
                            *optlen = core::mem::size_of::<u32>() as u32;
                        }
                        PosixSocketOption::SO_SNDBUF => {
                            *optval = SEND_BUFFER_SIZE as u32;
                            *optlen = core::mem::size_of::<u32>() as u32;
                        }
                        PosixSocketOption::SO_ERROR => {
                            *optval = 0;
                            *optlen = core::mem::size_of::<u32>() as u32;
                        }
                        opt => {
                            warn!(
                                    "[sys_getsockopt] unsupported SOL_SOCKET opt {opt:?} optlen:{optlen}"
                                )
                        }
                    },
                    None => {
                        warn!("[sys_getsockopt] unknown SOL_SOCKET opt {optname} optlen:{optlen}")
                    }
                }
            }
            SocketLevel::IPPROTO_IP | SocketLevel::IPPROTO_TCP => {
                const MAX_SEGMENT_SIZE: usize = 1666;
                match PosixTcpSocketOptions::from_repr(optname as i32) {
                    Some(opt) => match opt {
                        PosixTcpSocketOptions::MaxSegment => {
                            *optval = MAX_SEGMENT_SIZE as u32;
                            *optlen = core::mem::size_of::<u32>() as u32;
                        }
                        PosixTcpSocketOptions::NoDelay => {
                            *optval = 0;
                            *optlen = core::mem::size_of::<u32>() as u32;
                        }
                        PosixTcpSocketOptions::Info => {}
                        PosixTcpSocketOptions::Congestion => {
                            const CONGESTION: &str = "reno";
                            const CONGESTION_BYTES: &[u8] = CONGESTION.as_bytes();

                            let optval = UserPtr::<u8>::new(optval_ptr);
                            let buf_slice = optval.as_slice_mut_checked(CONGESTION.len()).await?;
                            buf_slice.copy_from_slice(CONGESTION_BYTES);
                            *optlen = CONGESTION.len() as u32;
                        }
                        opt => {
                            warn!(
                                "[sys_getsockopt] unsupported IPPROTO_TCP opt {opt:?} optlen:{optlen}"
                            )
                        }
                    },
                    None => {
                        warn!("[sys_getsockopt] unknown IPPROTO_TCP opt {optname} optlen:{optlen}")
                    }
                };
            }
            SocketLevel::IPPROTO_IPV6 => todo!(),
        }
        Ok(0)
    }

    /// receive data from a socket
    pub async fn sys_recvfrom(
        &self,
        sockfd: usize,
        buf: usize,
        len: usize,
        _flags: u32,
        addr: usize,
        addr_len: usize,
    ) -> SyscallResult {
        info!(
            "[sys_recvfrom] sockfd: {}, buf: {}, flags: ignored, addr: {}, addr_len: {}",
            sockfd, buf, addr, addr_len
        );

        //check addr len is valid
        if addr_len > 0 {
            if addr_len == 0xffffffffffffffff {
                return Err(Errno::EFAULT);
            }
            let addr_len_read = UserPtr::<i32>::new(addr_len).read().await?;
            if addr_len_read < 0 {
                return Err(Errno::EINVAL);
            }
        }

        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        let buf_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = buf_ptr.as_slice_mut_checked(len).await?;
        let (n, endpoint) = interruptable(self.task, socket.read(buf_slice), None, None).await?;
        drop(socket);

        let n = n?;

        let sockaddr = SockAddr::from_endpoint(endpoint.unwrap_or(IpEndpoint {
            addr: IpAddress::Ipv4(Ipv4Address::UNSPECIFIED),
            port: 0,
        }));
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        if user_ptr.is_non_null() {
            debug!("[sys_recvfrom] remote endpoint: {:?}", endpoint);
            warn!("[sys_recvfrom] remote Sockaddr's family: {:?}", unsafe {
                sockaddr.family
            });
            user_ptr.write(sockaddr).await?;
        } else {
            warn!(
                "[sys_recvfrom] addr pointer is null, not writing remote endpoint: {:?}",
                endpoint
            );
        }
        Ok(n as isize)
    }

    pub async fn sys_sendto(
        &self,
        sockfd: usize,
        buf: usize,
        len: usize,
        _flags: u32,
        addr: usize,
        addr_len: usize,
    ) -> SyscallResult {
        info!(
            "[sys_sendto] sockfd: {}, buf: {}, flags: ignored, addr: {}, addr_len: {}",
            sockfd, buf, addr, addr_len
        );

        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        let buf_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = buf_ptr.as_slice_const_checked(len).await?;
        let remote_endpoint = if addr == 0 {
            None
        } else {
            Some(SockAddr::new(addr, addr_len)?.get_endpoint())
        };
        let n = interruptable(
            self.task,
            socket.write(buf_slice, remote_endpoint),
            None,
            None,
        )
        .await??;
        drop(socket);

        Ok(n as isize)
    }

    pub async fn sys_shutdown(&self, sockfd: usize, how: usize) -> SyscallResult {
        info!("[sys_shutdown] sockfd: {}, how: {}", sockfd, how);

        let fd_table = self.task.fd_table();
        let socket_file = fd_table.get_socketfile(sockfd as usize)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        socket.shutdown(ShutdownType::from_bits_truncate(how as u8))?;

        Ok(0)
    }

    // socketpair now is like pipe
    pub async fn sys_socketpair(
        &self,
        _domain: isize,
        socket_type: isize,
        _protocol: isize,
        sv: usize,
    ) -> SyscallResult {
        // Extract flags from socket_type
        let flags = socket_type & !0xf; // Get the upper bits (flags)
        let has_nonblock = (flags & (SOCK_NONBLOCK as isize)) != 0;

        let mut file_flags = FileFlags::O_RDWR;
        if has_nonblock {
            file_flags |= FileFlags::O_NONBLOCK;
        }

        let (read_end, write_end) = PipeFile::new_pipe(&file_flags);

        let user_ptr = UserPtr::<i32>::new(sv);
        let buf_slice = user_ptr.as_slice_mut_checked(2).await?;

        let mut fd_table = self.task.fd_table();
        let read_fd = fd_table.alloc_fd()?;
        fd_table.set(read_fd, read_end);
        buf_slice[0] = read_fd as i32;

        let write_fd = fd_table.alloc_fd()?;
        fd_table.set(write_fd, write_end);
        buf_slice[1] = write_fd as i32;

        info!(
            "[sys_socketpair as sys_pipe2]: read fd {}, write fd {}",
            read_fd, write_fd
        );
        Ok(0)
    }

    // todo: implement sendmsg and recvmsg
    pub async fn sys_sendmsg(&self, socketfd: usize, msg_ptr: usize, flags: i32) -> SyscallResult {
        info!(
            "[sys_sendmsg]: socketfd: {}, msg_ptr: {}, flag: {}",
            socketfd, msg_ptr, flags
        );

        let _file = self.task.fd_table().get(socketfd).ok_or(Errno::EBADF)?;
        let ptr = UserPtr::<MessageHeaderRaw>::new(msg_ptr);
        let _message = ptr.read().await?;

        Ok(0)
    }

    pub async fn sys_sendmmsg(
        &self,
        socketfd: usize,
        msg_ptr: usize,
        n: usize,
        flags: i32,
    ) -> SyscallResult {
        info!(
            "[sys_sendmmsg]: socketfd: {}, msg_ptr: {}, flag: {}",
            socketfd, msg_ptr, flags
        );

        let _file = self.task.fd_table().get(socketfd).ok_or(Errno::EBADF)?;
        for i in 0..n {
            let ptr = UserPtr::<MessageHeaderRaw>::new(
                msg_ptr + i * core::mem::size_of::<MessageHeaderRaw>(),
            );
            let _message = ptr.read().await?;
        }

        Ok(0)
    }

    pub async fn sys_recvmsg(&self, socketfd: usize, msg_ptr: usize, flags: i32) -> SyscallResult {
        info!(
            "[sys_recvmsg]: socketfd: {}, msg_ptr: {}, flag: {}",
            socketfd, msg_ptr, flags
        );

        let _file = self.task.fd_table().get(socketfd).ok_or(Errno::EBADF)?;
        let ptr = UserPtr::<MessageHeaderRaw>::new(msg_ptr);
        let _message = ptr.read().await?;

        Ok(0)
    }

    pub async fn sys_recvmmsg(
        &self,
        socketfd: usize,
        msg_ptr: usize,
        n: usize,
        flags: i32,
    ) -> SyscallResult {
        info!(
            "[sys_recvmmsg]: socketfd: {}, msg_ptr: {}, flag: {}",
            socketfd, msg_ptr, flags
        );

        let _file = self.task.fd_table().get(socketfd).ok_or(Errno::EBADF)?;
        for i in 0..n {
            let ptr = UserPtr::<MessageHeaderRaw>::new(
                msg_ptr + i * core::mem::size_of::<MessageHeaderRaw>(),
            );
            let _message = ptr.read().await?;
        }

        Ok(0)
    }

    pub fn sys_setdomainname(&self, domainname: usize, len: usize) -> SyscallResult {
        info!(
            "[sys_setdomainname] domainname is {:?} len is {:?}",
            domainname, len
        );
        if (len as isize) < 0 || (len as isize) > 64 {
            return Err(Errno::EINVAL);
        }
        if (domainname as *const u8).is_null() {
            return Err(Errno::EFAULT);
        }

        let user_id = self.task.user_id();
        let (egid, euid) = (user_id.egid(), user_id.euid());
        debug!(
            "[sys_setdomainname]task egid {:?},task euid {:?}",
            egid, euid
        );
        if egid != 0 || euid != 0 {
            return Err(Errno::EPERM);
        }

        // let mut kernel_domainname: alloc::vec::Vec<u8> = vec![0; len];
        // copy_from_user(domainname, kernel_domainname.as_mut_ptr(), len)?;
        // error!("[sys_setdomainname] domainname is {:?}", kernel_domainname);

        // let file = path_openat("/etc/domainname", OpenFlags::O_CLOEXEC, AT_FDCWD,
        // 0)?; let offset = file.get_offset();
        // let clean_vec = vec![0; offset as usize];
        // file.pwrite(clean_vec.as_slice(), 0)?;
        // file.pwrite(kernel_domainname.as_slice(), 0)?;
        Ok(0)
    }

    pub fn sys_sethostname(&self, hostname: usize, len: usize) -> SyscallResult {
        info!(
            "[sys_sethostname] hostname is {:?} len is {:?}",
            hostname, len
        );
        if (len as isize) < 0 || (len as isize) > 64 {
            return Err(Errno::EINVAL);
        }
        if (hostname as *const u8).is_null() {
            return Err(Errno::EFAULT);
        }
        let user_id = self.task.user_id();
        let (egid, euid) = (user_id.egid(), user_id.euid());
        debug!(
            "[sys_setdomainname]task egid {:?},task euid {:?}",
            egid, euid
        );
        if egid != 0 || euid != 0 {
            return Err(Errno::EPERM);
        }
        // let mut kernel_hostname: Vec<u8> = vec![0; len];
        // copy_from_user(hostname, kernel_hostname.as_mut_ptr(), len)?;
        // error!("[sys_sethostname] hostname is {:?}", kernel_hostname);
        // let file = path_openat("/etc/hostname", OpenFlags::O_CLOEXEC, AT_FDCWD, 0)?;
        // file.pwrite(kernel_hostname.as_slice(), 0)?;
        Ok(0)
    }
}
