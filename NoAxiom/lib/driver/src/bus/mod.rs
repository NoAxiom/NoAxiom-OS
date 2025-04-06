use log::debug;

use crate::devices::ALL_DEVICES;

mod mmio;
mod pci;
mod pci_driver;

pub fn probe_bus() {
    debug!("[driver] probe mmio bus");
    ALL_DEVICES.ref_mut().probe_mmiobus_devices().ok();
    debug!("[driver] probe pci bus");
    ALL_DEVICES.ref_mut().probe_pcibus_devices().ok();
    debug!("[driver] probe succeed");
}
