use fdt::node::FdtNode;

use crate::{dtb::basic::DtbInfo, get_blk_dev};

mod plic;

pub fn arch_driver_init() {
    plic::init();
    plic::disable_blk_irq();
    #[cfg(feature = "intable")]
    {
        plic::enable_blk_irq();
    }
}

pub fn arch_handle_irq() {
    use arch::{Arch, ArchInt};
    assert!(!Arch::is_interrupt_enabled());
    let irq = plic::claim();
    log::error!("[driver] handle irq: {}", irq);
    if irq == 1 {
        get_blk_dev()
            .handle_interrupt()
            .expect("handle interrupt error");
    } else {
        log::error!("[driver] unhandled irq: {}", irq);
    }
    plic::complete(irq);
    log::error!("[driver] handle irq: {} finished", irq);
    assert!(!Arch::is_interrupt_enabled());
}

pub struct ArchDtbInfo {
    pub plic: usize,
}

impl ArchDtbInfo {
    pub fn new(plic: usize) -> Self {
        Self { plic }
    }
}

pub static ARCH_DTB_INITIALIZERS: &[fn(&FdtNode, &mut DtbInfo) -> bool] = &[plic::init_plic];
