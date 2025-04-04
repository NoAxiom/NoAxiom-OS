use alloc::string::{String, ToString};

use virtio_drivers::transport::pci::PciTransport;

pub trait Platform {
    fn model() -> Option<String>;
    fn get_dtb(dtb: usize) -> usize;
    fn memory_name() -> &'static str {
        "memory"
    }
    fn plic_name() -> &'static str {
        "plic"
    }
    fn clint_name() -> &'static str {
        "clint"
    }
    fn chose_name() -> &'static str {
        "chosen"
    }
    fn pci_name() -> &'static str {
        "pci"
    }
    fn virtio_mmio_name() -> &'static str {
        "virtio_mmio"
    }
    fn pci_init() -> Result<PciTransport, ()>;
}
