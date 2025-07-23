use fdt::node::FdtNode;

use crate::dtb::basic::DtbInfo;

pub const OF_PLIC_TYPE: &str = "riscv,plic0";

pub fn init_plic(node: &FdtNode, info: &mut DtbInfo) {
    let reg = node.reg().unwrap();
    reg.for_each(|x| info.arch.plic = x.starting_address as usize);
}
