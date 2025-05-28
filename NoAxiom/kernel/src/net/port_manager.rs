use alloc::{boxed::Box, collections::btree_map::BTreeMap};

use crate::{include::result::Errno, syscall::SysResult};

pub struct PortItem {
    pub fd: usize,
}

impl PortItem {
    pub fn new(fd: usize) -> Self {
        Self { fd }
    }
}

/// A table to record the listening endpoint of each port
/// for both TCP/UDP
///
/// `None` means the port is not listened  
/// `Some(x)` means the port is listened and the waker is used to wake up
/// the socket
pub struct PortManager {
    pub inner: Box<BTreeMap<u16, PortItem>>,
}

impl PortManager {
    pub fn new() -> Self {
        Self {
            inner: Box::new(BTreeMap::new()),
        }
    }

    /// Get an ephemeral port, for port 0
    pub fn get_ephemeral_port(&self) -> SysResult<u16> {
        // Ephemeral port range: 49152-65535
        static mut EPHEMERAL_PORT: u16 = 49152;
        for _ in 49152..65535 {
            let test_port = unsafe { EPHEMERAL_PORT };
            unsafe {
                EPHEMERAL_PORT += 1;
            }
            if self.inner.get(&test_port).is_none() {
                debug!("[port_manager] Get ephemeral port {test_port}");
                return Ok(test_port);
            }
        }
        Err(Errno::EADDRINUSE)
    }

    /// Bind a port with **SPECIFIC** port and fd
    pub fn bind_port_with_fd(&mut self, port: u16, fd: usize) -> SysResult<u16> {
        assert!(port < 49152, "Port number must be set");
        if let Some(_) = &self.inner.get(&port) {
            error!("[port_manager] Port {port} is already listened (with fd {fd})");
            Err(Errno::EADDRINUSE)
        } else {
            // let waker = current_task().unwrap().waker();
            self.inner.insert(port, PortItem::new(fd));
            Ok(port)
        }
    }

    /// Bind a port with a **RANDOM** port
    pub fn bind_port(&mut self, port: u16) -> SysResult<u16> {
        // port is u16 which is less than 65536
        assert!(
            49152 <= port,
            "Port number must be range from 49152 to 65535"
        );
        if let Some(_) = &self.inner.get(&port) {
            error!("[port_manager] Port {port} is already listened");
            Err(Errno::EADDRINUSE)
        } else {
            // let waker = current_task().unwrap().waker();
            self.inner.insert(port, PortItem::new(usize::MAX));
            Ok(port)
        }
    }

    pub fn unbind_port(&mut self, port: u16) {
        if let Some(_) = self.inner.remove(&port) {
            debug!("[port_manager] Unbind port {port}");
        } else {
            warn!("[port_manager] Port {port} is not listened");
        }
    }
}
