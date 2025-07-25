use crate::{basic::Device, DevResult};

pub mod plic;

pub trait InterruptDevice: Device {
    /// Handles the interrupt for this device.
    fn handle_irq(&self) -> DevResult<()>;
}
