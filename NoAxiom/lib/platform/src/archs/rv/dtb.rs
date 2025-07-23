use fdt::node::FdtNode;

use crate::{
    archs::rv::plic::{init_plic, OF_PLIC_TYPE},
    dtb::basic::DtbInfo,
};

/// Device Tree Base Address, at riscv64 is read from register
pub fn get_dtb(dtb: usize) -> usize {
    dtb
}

pub struct ArchDtbInfo {
    pub plic: usize,
}

impl ArchDtbInfo {
    pub fn new(plic: usize) -> Self {
        Self { plic }
    }
}

pub const ARCH_OF_INITIALIZERS: &[(&str, fn(&FdtNode, &mut DtbInfo))] =
    &[(OF_PLIC_TYPE, init_plic)];
