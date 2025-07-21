use fdt::node::FdtNode;

use crate::dtb::basic::DtbInfo;

pub fn arch_handle_irq() {}
pub fn arch_driver_init() {}
pub struct ArchDtbInfo {}
impl ArchDtbInfo {
    pub fn new(_opaque: usize) -> Self {
        Self {}
    }
}

pub static ARCH_DTB_INITIALIZERS: &[fn(&FdtNode, &mut DtbInfo) -> bool] = &[];
