use alloc::{boxed::Box, sync::Arc};
use core::task::Waker;

use async_trait::async_trait;
use ksync::async_mutex::{AsyncMutex, AsyncMutexGuard};
use smoltcp::wire::IpEndpoint;

use super::{
    socket::{poll_ifaces, SocketMeta},
    tcpsocket::TcpSocket,
    udpsocket::UdpSocket,
};
use crate::{
    fs::vfs::{
        basic::{
            dentry::{self, Dentry, DentryMeta, EmptyDentry},
            file::{File, FileMeta},
            inode::EmptyInode,
        },
        root_dentry,
    },
    include::{
        fs::{FileFlags, InodeMode},
        io::PollEvent,
        net::{AddressFamily, PosixSocketType, SockAddr, SocketOptions},
        result::Errno,
    },
    net::socket::Socket,
    sched::utils::block_on,
    syscall::{SysResult, SyscallResult},
    utils::random,
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
    pub fn setsockopt(&mut self, level: usize, optname: usize, optval: &[u8]) -> SysResult<()> {
        match self {
            Sock::Tcp(socket) => socket.setsockopt(level, optname, optval),
            Sock::Udp(socket) => socket.setsockopt(level, optname, optval), /* _ => Err(Errno::ENOSYS), */
        }
    }
    pub fn meta(&self) -> &SocketMeta {
        match self {
            Sock::Tcp(socket) => socket.meta(),
            Sock::Udp(socket) => socket.meta(),
        }
    }
}

pub struct SocketDentry {
    meta: DentryMeta,
}

impl SocketDentry {
    /// we mount all the pipes to the root dentry
    pub fn new(name: &str) -> Arc<Self> {
        let parent = root_dentry();
        let super_block = parent.super_block();
        let pipe_dentry = Arc::new(Self {
            meta: DentryMeta::new(Some(parent.clone()), name, super_block),
        });
        debug!(
            "[SocketDentry] create socket dentry: {}",
            pipe_dentry.name()
        );
        parent.add_child_directly(pipe_dentry.clone());
        pipe_dentry
    }
}

#[async_trait]
impl Dentry for SocketDentry {
    fn meta(&self) -> &DentryMeta {
        &self.meta
    }

    fn from_name(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        unreachable!("socket dentry should not have child");
    }

    fn open(self: Arc<Self>) -> SysResult<Arc<dyn File>> {
        unreachable!("socket dentry should not open");
    }

    async fn create(self: Arc<Self>, _name: &str, _mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        unreachable!("socket dentry should not create child");
    }
}

/// The file for socket
pub struct SocketFile {
    meta: FileMeta,
    sock: AsyncMutex<Sock>,
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

        let dentry = SocketDentry::new(&format!("socket-{}", random()));
        let empty_inode = EmptyInode::new();
        let meta = FileMeta::new(dentry, Arc::new(empty_inode));
        meta.set_flags(FileFlags::O_RDWR);

        Self {
            meta,
            sock: AsyncMutex::new(sock),
            type_,
        }
    }

    pub fn new_from_socket(socket: Arc<SocketFile>, sock: Sock) -> Self {
        let empty_dentry = EmptyDentry::new(&format!("socket-{}", random()));
        let empty_inode = EmptyInode::new();
        let meta = FileMeta::new(Arc::new(empty_dentry), Arc::new(empty_inode));
        meta.set_flags(FileFlags::O_RDWR);

        Self {
            meta,
            sock: AsyncMutex::new(sock),
            type_: socket.type_,
        }
    }

    pub async fn socket(&self) -> AsyncMutexGuard<'_, Sock> {
        self.sock.lock().await
    }
}

#[async_trait]
impl File for SocketFile {
    fn meta(&self) -> &FileMeta {
        &self.meta
    }
    async fn base_read(&self, _offset: usize, buf: &mut [u8]) -> SyscallResult {
        let mut sock = self.socket().await;
        let res;
        match &mut *sock {
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
        let mut sock = self.socket().await;
        let res;
        match &mut *sock {
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
        error!("Socket file is not a directory");
        Err(Errno::ENOTDIR)
    }
    /// Delete dentry, only for directories
    async fn delete_child(&self, _name: &str) -> Result<(), Errno> {
        unreachable!("Socket file is not a directory");
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> SyscallResult {
        warn!("[Socket::ioctl] not supported now, return 0 instead");
        Ok(0)
    }

    fn poll(&self, req: &PollEvent, waker: Waker) -> PollEvent {
        let mut sock = block_on(self.socket());
        poll_ifaces();
        let poll_res = match &mut *sock {
            Sock::Tcp(socket) => socket.poll(req, waker),
            Sock::Udp(socket) => socket.poll(),
        };

        let mut res = PollEvent::empty();
        if req.contains(PollEvent::POLLIN) && poll_res.contains(PollEvent::POLLIN) {
            res |= PollEvent::POLLIN;
        }
        if req.contains(PollEvent::POLLOUT) && poll_res.contains(PollEvent::POLLOUT) {
            res |= PollEvent::POLLOUT;
        }
        if poll_res.contains(PollEvent::POLLHUP) {
            warn!("[Socket::poll] PollEvent is hangup");
            res |= PollEvent::POLLHUP;
        }
        debug!("[Socket::poll] the result of PollEvent is {:?}", res);
        res
    }
}
