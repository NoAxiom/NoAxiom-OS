use ksync::mutex::SpinLock;

use crate::include::net::ShutdownType;

pub struct HandleItem {
    shutdown_type: SpinLock<ShutdownType>,
}

impl HandleItem {
    pub fn new() -> Self {
        Self {
            shutdown_type: SpinLock::new(ShutdownType::empty()),
        }
    }

    pub fn set_shutdown_type(&mut self, shutdown_type: ShutdownType) {
        *self.shutdown_type.lock() = shutdown_type;
    }

    pub fn get_shutdown_type(&self) -> ShutdownType {
        self.shutdown_type.lock().clone()
    }
}
