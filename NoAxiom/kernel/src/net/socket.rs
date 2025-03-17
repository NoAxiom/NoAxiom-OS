use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use smoltcp::wire::IpEndpoint;

use crate::{fs::vfs::basic::file::File, include::net::ShutdownType, syscall::SysResult};

/// TCP/UDP or other socket should implement this trait
#[async_trait]
// pub trait Socket: File {
pub trait Socket: Send + Sync {
    /// The bind() function is used to associate a socket with a particular IP
    /// address and port number on the local machine.
    ///
    /// return: whether the operation is successful
    fn bind(&mut self, local: IpEndpoint) -> SysResult<usize>;

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
