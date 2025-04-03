use alloc::string::{String, ToString};

use virtio_drivers::transport::pci::PciTransport;

pub trait Platform {
    fn model() -> Option<String>;
    fn get_dtb(dtb: usize) -> usize;
    fn memory_name() -> String {
        "memory".to_string()
    }
    fn plic_name() -> String {
        "plic".to_string()
    }
    fn clint_name() -> String {
        "clint".to_string()
    }
    fn chose_name() -> String {
        "chosen".to_string()
    }
    fn pci_init() -> Result<PciTransport, ()>;
}
