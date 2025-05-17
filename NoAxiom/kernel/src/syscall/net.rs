//! todo: Network socket struct has no lock now
use alloc::sync::Arc;

use super::SyscallResult;
use crate::{
    fs::pipe::PipeFile,
    include::{
        net::{AddressFamily, PosixSocketType, SockAddr},
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
        let sock_addr = user_ptr.try_read().await?;

        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::ENOTSOCK)?;
        drop(fd_table);

        let mut socket = socket_file.socket();
        socket.bind(sock_addr)?;

        Ok(0)
    }

    pub fn sys_listen(&self, sockfd: usize, backlog: usize) -> SyscallResult {
        debug!("[sys_listen] sockfd: {}, backlog: {}", sockfd, backlog);
        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::ENOTSOCK)?;
        drop(fd_table);

        let mut socket = socket_file.socket();
        socket.listen(backlog)?;

        Ok(0)
    }

    pub async fn sys_connect(&self, sockfd: usize, addr: usize, _addrlen: usize) -> SyscallResult {
        debug!("[sys_connect] sockfd: {}, addr: {}", sockfd, addr);
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        let sock_addr = user_ptr.try_read().await?;

        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::ENOTSOCK)?;
        drop(fd_table);

        let mut socket = socket_file.socket();
        socket.connect(sock_addr).await?;

        Ok(0)
    }

    pub async fn sys_accept(&self, sockfd: usize, addr: usize, _addrlen: usize) -> SyscallResult {
        let fd_table = self.task.fd_table();
        let socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::ENOTSOCK)?;
        drop(fd_table);

        let mut socket = socket_file.socket();
        let (new_tcp_socket, endpoint) = socket.accept().await?;

        let sockaddr = SockAddr::from_endpoint(endpoint);
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        user_ptr.try_write(sockaddr).await?;

        let new_socket_file =
            SocketFile::new_from_socket(socket_file.clone(), Sock::Tcp(new_tcp_socket));
        let mut fd_table = self.task.fd_table();
        let new_fd = fd_table.alloc_fd()?;
        fd_table.set(new_fd as usize, Arc::new(new_socket_file));

        Ok(new_fd as isize)
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
