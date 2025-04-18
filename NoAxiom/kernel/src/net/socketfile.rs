use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use smoltcp::wire::IpEndpoint;

use super::{tcpsocket::TcpSocket, udpsocket::UdpSocket};
use crate::{
    fs::vfs::basic::{
        dentry::EmptyDentry,
        file::{File, FileMeta},
        inode::EmptyInode,
    },
    include::{
        fs::FileFlags,
        net::{AddressFamily, PosixSocketType, SockAddr, SocketOptions},
        result::Errno,
    },
    net::socket::Socket,
    syscall::{SysResult, SyscallResult},
};

pub enum Sock {
    Tcp(TcpSocket),
    Udp(UdpSocket),
    // Unix(UnixSocket),
}

impl Sock {
    pub fn bind(&mut self, addr: SockAddr) -> SysResult<()> {
        let endpoint = addr.get_endpoint();
        match self {
            Sock::Tcp(socket) => socket.bind(endpoint).map_err(|_| Errno::EINVAL),
            Sock::Udp(socket) => socket.bind(endpoint).map_err(|_| Errno::EINVAL),
            // Sock::Unix(unix_sock) => unix_sock.bind().map_err(|_| Errno::EINVAL),
        }
    }
    pub fn listen(&mut self, backlog: usize) -> SysResult<()> {
        match self {
            Sock::Tcp(socket) => socket.listen(backlog).map_err(|_| Errno::EINVAL),
            _ => Err(Errno::ENOSYS),
        }
    }
    pub async fn connect(&mut self, addr: SockAddr) -> SysResult<()> {
        let endpoint = addr.get_endpoint();
        match self {
            Sock::Tcp(socket) => socket.connect(endpoint).await.map_err(|_| Errno::EINVAL),
            Sock::Udp(socket) => socket.connect(endpoint).await.map_err(|_| Errno::EINVAL),
            // _ => Err(Errno::ENOSYS),
        }
    }
    pub async fn accept(&mut self) -> SysResult<(TcpSocket, IpEndpoint)> {
        match self {
            Sock::Tcp(socket) => {
                let (new_socket, endpoint) = socket.accept().await.map_err(|_| Errno::EINVAL)?;
                Ok((new_socket, endpoint))
            }
            _ => Err(Errno::ENOSYS),
        }
    }
}

/// No lock File struct for managing socket
pub struct SocketFile {
    meta: FileMeta,
    sock: Sock,
    type_: PosixSocketType,
}

unsafe impl Send for SocketFile {}
unsafe impl Sync for SocketFile {}

impl SocketFile {
    pub fn new(addr_family: AddressFamily, type_: PosixSocketType) -> Self {
        let sock = match addr_family {
            AddressFamily::AF_INET | AddressFamily::AF_INET6 => match type_ {
                PosixSocketType::Stream => Sock::Tcp(TcpSocket::new(SocketOptions::default())),
                PosixSocketType::Datagram => Sock::Udp(UdpSocket::new(SocketOptions::default())),
                _ => unimplemented!("Unsupported socket type"),
            },
            AddressFamily::AF_UNIX => todo!("Unsupported address family AF_UNIX"),
        };

        let empty_dentry = EmptyDentry::new();
        let empty_inode = EmptyInode::new();
        let meta = FileMeta::new(Arc::new(empty_dentry), Arc::new(empty_inode));
        meta.set_flags(FileFlags::O_RDWR);

        Self { meta, sock, type_ }
    }

    pub fn new_from_socket(socket: Arc<SocketFile>, sock: Sock) -> Self {
        let empty_dentry = EmptyDentry::new();
        let empty_inode = EmptyInode::new();
        let meta = FileMeta::new(Arc::new(empty_dentry), Arc::new(empty_inode));
        meta.set_flags(FileFlags::O_RDWR);

        Self {
            meta,
            sock,
            type_: socket.type_,
        }
    }

    pub fn socket(&self) -> &Sock {
        &self.sock
    }

    pub fn socket_mut(&mut self) -> &mut Sock {
        &mut self.sock
    }
}

#[async_trait]
impl File for SocketFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, _offset: usize, buf: &mut [u8]) -> SyscallResult {
        let sock = self.socket();
        let res;
        match sock {
            Sock::Tcp(socket) => {
                res = socket.read(buf).await.0?;
            }
            Sock::Udp(socket) => {
                res = socket.read(buf).await.0?;
            } /* Sock::Unix(unix_sock) => unix_sock.base_read(buf).await,
               * _ => unimplemented!("Unsupported socket type"), */
        }

        Ok(res as isize)
    }
    async fn base_readlink(&self, _buf: &mut [u8]) -> SyscallResult {
        unreachable!()
    }
    async fn base_write(&self, _offset: usize, buf: &[u8]) -> SyscallResult {
        let sock = self.socket();
        let res;
        match sock {
            Sock::Tcp(socket) => {
                res = socket.write(buf, None).await?;
            }
            Sock::Udp(socket) => {
                // use its remote endpoint as the destination
                res = socket.write(buf, None).await?;
            } /* Sock::Unix(unix_sock) => unix_sock.base_read(buf).await,
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
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        warn!("[Socket::ioctl] not supported now, return 0 instead");
        Ok(0)
    }
}
