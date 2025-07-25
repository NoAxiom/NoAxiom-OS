use crate::{basic::Device, devices::get_intr_dev};

pub mod plic;

pub trait InterruptDevice: Device {
    /// Handles the interrupt for this device.
    fn handle_irq(&self);
}

pub fn handle_irq() {
    if let Some(dev) = get_intr_dev() {
        dev.handle_irq();
    }
}
