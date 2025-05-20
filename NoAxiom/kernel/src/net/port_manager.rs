//! IP layer
use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use core::task::Waker;

use futures::task::noop_waker;

use crate::{cpu::current_task, include::result::Errno, syscall::SysResult};

pub struct PortItem {
    waker: Waker,
}

impl PortItem {
    pub fn new(waker: Waker) -> Self {
        Self { waker }
    }
}

impl Default for PortItem {
    fn default() -> Self {
        Self {
            waker: noop_waker(),
        }
    }
}

const EMPTY_PORT_ITEM: Option<PortItem> = None;

/// A table to record the listening endpoint of each port
/// for both TCP/UDP
///
/// `None` means the port is not listened  
/// `Some(x)` means the port is listened and the waker is used to wake up
/// the socket
pub struct PortManager {
    inner: Box<BTreeMap<usize, PortItem>>,
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
            if self.inner.get(&(test_port as usize)).is_none() {
                debug!("[port_manager] Get ephemeral port {test_port}");
                return Ok(test_port);
            }
        }
        Err(Errno::EADDRINUSE)
    }

    /// Bind a port with a socket
    pub fn bind_port(&mut self, port: u16) -> SysResult<()> {
        let port = if port == 0 {
            self.get_ephemeral_port()?
        } else {
            port
        };
        if let Some(_) = &self.inner.get(&(port as usize)) {
            // The port is already listened
            error!("[port_manager] Port {port} is already listened");
            Err(Errno::EADDRINUSE)
        } else {
            // let waker = current_task().unwrap().waker();
            let waker = noop_waker();
            self.inner.insert(port as usize, PortItem::new(waker));
            Ok(())
        }
    }
}
