use crate::include::net::ShutdownType;

pub struct HandleItem {
    shutdown_type: ShutdownType,
}

impl HandleItem {
    pub fn new() -> Self {
        Self {
            shutdown_type: ShutdownType::empty(),
        }
    }

    pub fn set_shutdown_type(&mut self, shutdown_type: ShutdownType) {
        self.shutdown_type = shutdown_type;
    }

    pub fn get_shutdown_type(&self) -> &ShutdownType {
        &self.shutdown_type
    }
}
