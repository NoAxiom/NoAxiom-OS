use virtio_drivers::transport::pci::PciTransport;

use crate::{rv64::RV64, Platform};

impl Platform for RV64 {
    fn get_dtb(dtb: usize) -> usize {
        dtb
    }
    fn pci_init() -> Result<PciTransport, ()> {
        unreachable!()
    }
}
