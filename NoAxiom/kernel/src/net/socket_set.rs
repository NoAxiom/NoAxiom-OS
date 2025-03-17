use ksync::mutex::SpinLock;
use smoltcp::{iface::SocketSet as SmoltcpSocketSet, socket::AnySocket};

pub struct SocketSet {
    inner: SpinLock<SmoltcpSocketSet<'static>>,
}

impl SocketSet {
    pub fn new() -> Self {
        Self {
            inner: SpinLock::new(SmoltcpSocketSet::new(vec![])),
        }
    }

    pub fn insert<S>(&self, socket: S)
    where
        S: AnySocket<'static>,
    {
        self.inner.lock().add(socket);
    }
}
