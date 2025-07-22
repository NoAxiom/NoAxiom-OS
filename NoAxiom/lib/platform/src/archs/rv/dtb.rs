use fdt::node::FdtNode;

use crate::{archs::consts::PLIC_NAME, dtb::basic::DtbInfo};

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

pub static ARCH_DTB_INITIALIZERS: &[fn(&FdtNode, &mut DtbInfo) -> bool] = &[init_plic];

pub fn init_plic(node: &FdtNode, info: &mut DtbInfo) -> bool {
    if node.name.starts_with(PLIC_NAME) {
        let reg = node.reg().unwrap();
        reg.for_each(|x| info.arch.plic = x.starting_address as usize);
        true
    } else {
        false
    }
}
