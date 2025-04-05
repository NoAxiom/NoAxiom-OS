use alloc::string::String;

/// Arch related information for device tree blob
pub trait DtbInfo {
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
}
