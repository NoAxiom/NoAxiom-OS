use crate::bus::pci::probe_pci_bus;

pub(crate) mod pci;
pub(crate) mod pci_driver;

pub fn bus_init() {
    probe_pci_bus();
}
