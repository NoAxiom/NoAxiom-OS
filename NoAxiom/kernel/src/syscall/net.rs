//! todo: Network socket struct has no lock now
use alloc::sync::Arc;

use super::SyscallResult;
use crate::{
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

        Err(Errno::ENOSYS)
    }

    pub fn sys_bind(&self, sockfd: usize, addr: usize, _addr_len: usize) -> SyscallResult {
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        let sock_addr = user_ptr.read();

        let fd_table = self.task.fd_table();
        let mut socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::ENOTSOCK)?;
        drop(fd_table);

        let socket = Arc::get_mut(&mut socket_file)
            .ok_or(Errno::EBADF)?
            .socket_mut();
        socket.bind(sock_addr)?;

        Ok(0)
    }

    pub fn sys_listen(&self, sockfd: usize, backlog: usize) -> SyscallResult {
        let fd_table = self.task.fd_table();
        let mut socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::ENOTSOCK)?;
        drop(fd_table);

        let socket = Arc::get_mut(&mut socket_file)
            .ok_or(Errno::EBADF)?
            .socket_mut();
        socket.listen(backlog)?;

        Ok(0)
    }

    pub async fn sys_connect(&self, sockfd: usize, addr: usize, _addrlen: usize) -> SyscallResult {
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        let sock_addr = user_ptr.read();

        let fd_table = self.task.fd_table();
        let mut socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::ENOTSOCK)?;
        drop(fd_table);

        let socket = Arc::get_mut(&mut socket_file)
            .ok_or(Errno::EBADF)?
            .socket_mut();
        socket.connect(sock_addr).await?;

        Ok(0)
    }

    pub async fn sys_accept(&self, sockfd: usize, addr: usize, _addrlen: usize) -> SyscallResult {
        let fd_table = self.task.fd_table();
        let mut socket_file = fd_table
            .get_socketfile(sockfd as usize)
            .ok_or(Errno::ENOTSOCK)?;
        drop(fd_table);

        let socket = Arc::get_mut(&mut socket_file)
            .ok_or(Errno::EBADF)?
            .socket_mut();
        let (new_tcp_socket, endpoint) = socket.accept().await?;

        let sockaddr = SockAddr::from_endpoint(endpoint);
        let user_ptr = UserPtr::<SockAddr>::new(addr);
        user_ptr.write(sockaddr);

        let new_socket_file = SocketFile::new_from_socket(socket_file, Sock::Tcp(new_tcp_socket));
        let mut fd_table = self.task.fd_table();
        let new_fd = fd_table.alloc_fd()?;
        fd_table.set(new_fd as usize, Arc::new(new_socket_file));

        Ok(new_fd as isize)
    }
}
