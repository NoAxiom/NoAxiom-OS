use fdt::node::FdtNode;

use crate::{dtb::basic::DtbInfo, PLIC_NAME};

pub mod base;

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
