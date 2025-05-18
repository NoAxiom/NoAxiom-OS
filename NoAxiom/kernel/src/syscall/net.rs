//! todo: Network socket struct has no lock now
use alloc::sync::Arc;

use super::SyscallResult;
use crate::{
    constant::net::SOL_SOCKET,
    fs::pipe::PipeFile,
    include::{
        net::{
            AddressFamily, PosixIpProtocol, PosixSocketOption, PosixSocketType,
            PosixTcpSocketOptions, SockAddr,
        },
        result::Errno,
    },
    mm::user_ptr::UserPtr,
    net::socketfile::{Sock, SocketFile},
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

        // todo: maybe should add socket inode
        let socket_file = SocketFile::new(
            AddressFamily::try_from(address_family as u16)?,
            PosixSocketType::try_from(socket_type & 0xf)?,
        );

        let mut fd_table = self.task.fd_table();
        let socket_fd = fd_table.alloc_fd()?;
        fd_table.set(socket_fd as usize, Arc::new(socket_file));

        Ok(socket_fd as isize)
    }

    pub async fn sys_bind(&self, sockfd: usize, addr: usize, _addr_len: usize) -> SyscallResult {
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        let sock_addr = user_ptr.read().await?;

        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::EBADF)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        socket.bind(sock_addr)?;

        Ok(0)
    }

    pub async fn sys_listen(&self, sockfd: usize, backlog: usize) -> SyscallResult {
        debug!("[sys_listen] sockfd: {}, backlog: {}", sockfd, backlog);
        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::EBADF)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        socket.listen(backlog)?;

        Ok(0)
    }

    pub async fn sys_connect(&self, sockfd: usize, addr: usize, _addrlen: usize) -> SyscallResult {
        debug!("[sys_connect] sockfd: {}, addr: {}", sockfd, addr);
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        let sock_addr = user_ptr.read().await?;

        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::EBADF)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        socket.connect(sock_addr).await?;

        Ok(0)
    }

    pub async fn sys_accept(&self, sockfd: usize, addr: usize, _addrlen: usize) -> SyscallResult {
        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::EBADF)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;
        let (new_tcp_socket, endpoint) = socket.accept().await?;

        let sockaddr = SockAddr::from_endpoint(endpoint);
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        user_ptr.write(sockaddr).await?;

        let new_socket_file =
            SocketFile::new_from_socket(socket_file.clone(), Sock::Tcp(new_tcp_socket));
        let mut fd_table = self.task.fd_table();
        let new_fd = fd_table.alloc_fd()?;
        fd_table.set(new_fd as usize, Arc::new(new_socket_file));

        Ok(new_fd as isize)
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
        info!(
            "[sys_setsockopt] sockfd: {:?}, level: {:?}, optname: {:?}, optval_ptr: {:?}, optlen: {:?}",
            sockfd, level, optname, optval_ptr, optlen
        );

        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::EBADF)?;
        drop(fd_table);

        let mut socket = socket_file.socket().await;

        let user_ptr = UserPtr::<u8>::new(optval_ptr);
        let buf_slice = user_ptr.as_slice_mut_checked(optlen).await?;
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

        let optval = UserPtr::<u32>::new(optval_ptr);
        let optval = optval.get_ref_mut().await?.ok_or(Errno::EFAULT)?;
        let optlen = UserPtr::<u32>::new(optlen);
        let optlen = optlen.get_ref_mut().await?.ok_or(Errno::EFAULT)?;

        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::EBADF)?;
        drop(fd_table);

        let socket = socket_file.socket().await;

        if level as u8 == SOL_SOCKET {
            let optname = PosixSocketOption::from_repr(optname as i32).ok_or(Errno::ENOPROTOOPT)?;
            match optname {
                PosixSocketOption::SO_SNDBUF => {
                    *optval = socket.meta().tx_buf_size as u32;
                    *optlen = core::mem::size_of::<u32>() as u32;
                    return Ok(0);
                }
                PosixSocketOption::SO_RCVBUF => {
                    *optval = socket.meta().rx_buf_size as u32;
                    *optlen = core::mem::size_of::<u32>() as u32;
                    return Ok(0);
                }
                _ => {
                    return Err(Errno::ENOPROTOOPT);
                }
            }
        }
        drop(socket);

        // To manipulate options at any other level the
        // protocol number of the appropriate protocol controlling the
        // option is supplied.  For example, to indicate that an option is
        // to be interpreted by the TCP protocol, level should be set to the
        // protocol number of TCP.

        let posix_protocol = PosixIpProtocol::from_repr(level as u16).ok_or(Errno::ENOPROTOOPT)?;
        if posix_protocol == PosixIpProtocol::TCP {
            let optname =
                PosixTcpSocketOptions::from_repr(optname as i32).ok_or(Errno::ENOPROTOOPT)?;
            match optname {
                PosixTcpSocketOptions::Congestion => return Ok(0),
                _ => {
                    return Err(Errno::ENOPROTOOPT);
                }
            }
        }
        return Err(Errno::ENOPROTOOPT);
    }

    // socketpair now is like pipe
    pub async fn sys_socketpair(
        &self,
        _domain: isize,
        _type: isize,
        _protocol: isize,
        sv: usize,
    ) -> SyscallResult {
        let (read_end, write_end) = PipeFile::new_pipe();

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
}
