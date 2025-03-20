//! IP layer
use alloc::sync::Arc;
use core::task::Waker;

use ksync::mutex::SpinLock;
use smoltcp::{socket::AnySocket, wire::IpListenEndpoint};

use crate::{constant::net::PORT_MAX, include::result::Errno, syscall::SysResult};

#[derive(Clone, Copy)]
pub struct PortItem {}

impl PortItem {
    pub fn new() -> Self {
        Self {}
    }
}

/// A table to record the listening endpoint of each port
/// for both TCP/UDP
///
/// `None` means the port is not listened  
/// `Some(x)` means the port is listened and the waker is used to wake up
/// the socket
pub struct PortManager {
    inner: Arc<SpinLock<[Option<PortItem>; PORT_MAX]>>,
}

impl PortManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SpinLock::new([None; PORT_MAX])),
        }
    }

    /// Get an ephemeral port, for port 0
    pub fn get_ephemeral_port(&self) -> SysResult<u16> {
        // Ephemeral port range: 49152-65535
        static mut EPHEMERAL_PORT: u16 = 49152;
        let inner = self.inner.lock();
        for _ in 49152..65535 {
            let test_port = unsafe { EPHEMERAL_PORT };
            unsafe {
                EPHEMERAL_PORT += 1;
            }
            if inner[test_port as usize].is_none() {
                return Ok(test_port);
            }
        }
        Err(Errno::EADDRINUSE)
    }

    /// Bind a port with a socket
    pub fn bind_port<S>(&self, port: u16) -> SysResult<()>
    where
        S: AnySocket<'static>,
    {
        let port = if port == 0 {
            self.get_ephemeral_port()?
        } else {
            port
        };
        let mut inner = self.inner.lock();
        if let Some(_) = &inner[port as usize] {
            // The port is already listened
            Err(Errno::EADDRINUSE)
        } else {
            inner[port as usize] = Some(PortItem::new());
            Ok(())
        }
    }
}
