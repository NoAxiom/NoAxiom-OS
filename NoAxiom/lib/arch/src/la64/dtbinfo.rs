use alloc::string::ToString;

use super::LA64;
use crate::{la64::memory::KERNEL_ADDR_OFFSET, DtbInfo};

const QEMU_DTB_ADDR: usize = 0x100000;

impl DtbInfo for LA64 {
    fn model() -> Option<alloc::string::String> {
        Some("loongarch64-qemu".to_string())
    }
    fn get_dtb(_dtb: usize) -> usize {
        QEMU_DTB_ADDR | KERNEL_ADDR_OFFSET
    }
    fn plic_name() -> &'static str {
        "platic"
    }
    // Core Local Interruptor
    fn clint_name() -> &'static str {
        // "cpuic".to_string()
        "eiointc"
    }
}
