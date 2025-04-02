use alloc::string::ToString;

use virtio_drivers::transport::pci::PciTransport;

use super::LA64;
use crate::{la64::memory::KERNEL_ADDR_OFFSET, Platform};

const QEMU_DTB_ADDR: usize = 0x100000;

impl Platform for LA64 {
    fn model() -> Option<alloc::string::String> {
        Some("loongarch64-qemu".to_string())
    }
    fn get_dtb() -> usize {
        QEMU_DTB_ADDR | KERNEL_ADDR_OFFSET
    }
    fn plic_name() -> alloc::string::String {
        "platic".to_string()
    }
    // Core Local Interruptor
    fn clint_name() -> alloc::string::String {
        // "cpuic".to_string()
        "eiointc".to_string()
    }
    fn pci_init() -> Result<PciTransport, ()> {
        // poly::pci::init()
        Err(())
    }
}
