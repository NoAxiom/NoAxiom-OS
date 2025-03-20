use ksync::mutex::{SpinLock, SpinLockGuard};
use smoltcp::{
    iface::{SocketHandle, SocketSet as SmoltcpSocketSet},
    socket::AnySocket,
};

pub struct SocketSet {
    inner: SpinLock<SmoltcpSocketSet<'static>>,
}

impl SocketSet {
    pub fn new() -> Self {
        Self {
            inner: SpinLock::new(SmoltcpSocketSet::new(vec![])),
        }
    }

    /// Insert a socket into the socket set, returning a handle to it.
    pub fn insert<S>(&self, socket: S) -> SocketHandle
    where
        S: AnySocket<'static>,
    {
        self.inner.lock().add(socket)
    }

    pub fn lock(&self) -> SpinLockGuard<SmoltcpSocketSet<'static>> {
        self.inner.lock()
    }
}
