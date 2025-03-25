use alloc::sync::Arc;

use super::SyscallResult;
use crate::{
    include::{
        net::{AddressFamily, PosixSocketType},
        result::Errno,
    },
    net::sockfile::SocketFile,
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
}
