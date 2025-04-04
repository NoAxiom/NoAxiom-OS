use crate::devices::ALL_DEVICES;

mod mmio;
mod pci;
mod pci_driver;

pub fn probe_bus() {
    ALL_DEVICES.ref_mut().probe_mmiobus_devices().ok();
    ALL_DEVICES.ref_mut().probe_pcibus_devices().ok();
}
