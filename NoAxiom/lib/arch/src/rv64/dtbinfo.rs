use alloc::string::String;

use virtio_drivers::transport::pci::PciTransport;

use crate::{rv64::RV64, DtbInfo};

impl DtbInfo for RV64 {
    fn model() -> Option<String> {
        None
    }
    fn get_dtb(dtb: usize) -> usize {
        dtb
    }
}
