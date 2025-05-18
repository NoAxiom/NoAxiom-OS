use log::debug;

use crate::devices::{impls::net::loopback::LoopBackDev, ALL_DEVICES};

mod mmio;
mod pci;
mod pci_driver;

pub fn probe_bus() {
    debug!("[driver] probe mmio bus");
    ALL_DEVICES.as_ref_mut().probe_mmiobus_devices().ok();
    debug!("[driver] probe pci bus");
    ALL_DEVICES.as_ref_mut().probe_pcibus_devices().ok();
    ALL_DEVICES.as_ref_mut().add_net_device(LoopBackDev::new());
    debug!("[driver] probe succeed");
}
