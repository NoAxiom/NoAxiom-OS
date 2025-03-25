use alloc::{boxed::Box, sync::Arc};
use core::ops::Deref;

use async_trait::async_trait;
use ksync::mutex::SpinLock;

use super::tcpsocket::TcpSocket;
use crate::{
    fs::vfs::basic::{
        dentry::{self, EmptyDentry},
        file::{File, FileMeta},
        inode::EmptyInode,
    },
    include::{
        fs::FileFlags,
        net::{AddressFamily, PosixSocketType, SocketOptions, SocketType},
        result::Errno,
    },
    net::socket::Socket,
    syscall::SyscallResult,
};

pub enum Sock {
    Tcp(TcpSocket),
    // Udp(UdpSocket),
    // Unix(UnixSocket),
}

pub struct SocketFile {
    meta: FileMeta,
    sock: Arc<SpinLock<Sock>>,
    type_: PosixSocketType,
}

unsafe impl Send for SocketFile {}
unsafe impl Sync for SocketFile {}

impl SocketFile {
    pub fn new(addr_family: AddressFamily, type_: PosixSocketType) -> Self {
        let sock = match addr_family {
            AddressFamily::AF_INET | AddressFamily::AF_INET6 => match type_ {
                PosixSocketType::Stream => Sock::Tcp(TcpSocket::new(SocketOptions::default())),
                // SocketType::Datagram => Sock::Udp(UdpSocket::new(SocketOptions::default())),
                _ => unimplemented!("Unsupported socket type"),
            },
            AddressFamily::AF_UNIX => todo!("Unsupported address family AF_UNIX"),
        };

        let empty_dentry = EmptyDentry::new();
        let empty_inode = EmptyInode::new();
        let meta = FileMeta::new(Arc::new(empty_dentry), Arc::new(empty_inode));
        meta.set_flags(FileFlags::O_RDWR);

        Self {
            meta,
            sock: Arc::new(SpinLock::new(sock)),
            type_,
        }
    }
}

#[async_trait]
impl File for SocketFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, _offset: usize, buf: &mut [u8]) -> SyscallResult {
        let sock_guard = self.sock.lock();
        let sock = sock_guard.deref();
        let res;
        match sock {
            Sock::Tcp(socket) => {
                res = socket.read(buf).await.0?;
            } /* Sock::Udp(udp_sock) => udp_sock.base_read(buf).await,
               * Sock::Unix(unix_sock) => unix_sock.base_read(buf).await,
               * _ => unimplemented!("Unsupported socket type"), */
        }

        Ok(res as isize)
    }
    async fn base_write(&self, _offset: usize, buf: &[u8]) -> SyscallResult {
        let sock_guard = self.sock.lock();
        let sock = sock_guard.deref();
        let res;
        match sock {
            Sock::Tcp(socket) => {
                res = socket.write(buf, None).await?;
            } /* Sock::Udp(udp_sock) => udp_sock.base_write(buf).await,
               * Sock::Unix(unix_sock) => unix_sock.base_write(buf).await,
               * _ => unimplemented!("Unsupported socket type"), */
        }

        Ok(res as isize)
    }
    /// Load directory into memory, must be called before read/write explicitly,
    /// only for directories
    async fn load_dir(&self) -> Result<(), Errno> {
        unreachable!("Socket file is not a directory");
    }
    /// Delete dentry, only for directories
    async fn delete_child(&self, _name: &str) -> Result<(), Errno> {
        unreachable!("Socket file is not a directory");
    }
}
