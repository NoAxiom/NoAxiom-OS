use alloc::string::String;

use virtio_drivers::transport::pci::PciTransport;

use crate::{rv64::RV64, Platform};

impl Platform for RV64 {
    fn model() -> Option<String> {
        None
    }
    fn get_dtb() -> usize {
        0xffffffc087000000
    }
    fn pci_init() -> Result<PciTransport, ()> {
        unreachable!()
    }
}
