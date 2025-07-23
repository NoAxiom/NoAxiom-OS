use fdt::node::FdtNode;

use crate::dtb::basic::DtbInfo;

/// Device Tree Base Address, at loongarch64 is a constant
pub fn get_dtb(_dtb: usize) -> usize {
    0x100000
}

pub struct ArchDtbInfo {}
impl ArchDtbInfo {
    pub fn new(_opaque: usize) -> Self {
        Self {}
    }
}

pub const ARCH_OF_INITIALIZERS: &[(&str, fn(&FdtNode, &mut DtbInfo))] = &[];
