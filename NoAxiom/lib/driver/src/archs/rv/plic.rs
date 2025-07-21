extern crate alloc;

use arch::{Arch, ArchAsm, ArchMemory};
use config::cpu::CPU_NUM;
use fdt::node::FdtNode;
use ksync::Once;
use log::debug;
use plic::{Mode, PLIC};

use crate::dtb::basic::{dtb_info, DtbInfo};

pub static PLIC: Once<PLIC<CPU_NUM>> = Once::new();

pub fn disable_blk_irq() {
    let plic = unsafe { PLIC.get_unchecked() };
    let irq = 1;
    let hart = Arch::get_hartid();
    plic.disable(hart as u32, Mode::Supervisor, irq);
}

pub fn enable_blk_irq() {
    let plic = unsafe { PLIC.get_unchecked() };
    let irq = 1;
    let hart = Arch::get_hartid();
    plic.enable(hart as u32, Mode::Supervisor, irq);
}

pub fn claim() -> u32 {
    let plic = unsafe { PLIC.get_unchecked() };
    let hart = Arch::get_hartid();
    plic.claim(hart as u32, Mode::Supervisor)
}

pub fn complete(irq: u32) {
    let plic = unsafe { PLIC.get_unchecked() };
    let hart = Arch::get_hartid();
    plic.complete(hart as u32, Mode::Supervisor, irq);
}

pub fn init() {
    let plic_addr = dtb_info().arch.plic | arch::Arch::KERNEL_ADDR_OFFSET;
    debug!("PLIC addr: {:#x}", plic_addr);
    let privileges = [2; CPU_NUM];
    let plic = PLIC::new(plic_addr, privileges);
    PLIC.call_once(|| plic);

    let priority = 1;
    let irq = 1;
    let plic = unsafe { PLIC.get_unchecked() };
    plic.set_priority(irq, priority);

    // todo: register more devices
    log::info!("Init plic success");
    #[cfg(any(feature = "vf2"))]
    {
        let mut privileges = [2; CPU_NUM];
        // core 0 don't have S mode
        privileges[0] = 1;
        println!("PLIC context: {:?}", privileges);
        let plic = PLIC::new(plic_addr, privileges);
        PLIC.call_once(|| plic);
        println!("Init hifive or vf2 plic success");
    }

    for i in 0..CPU_NUM {
        register_to_hart(i as u32);
    }
}

pub fn register_to_hart(hart: u32) {
    let plic = unsafe { PLIC.get_unchecked() };
    let irq = 1;
    plic.enable(hart, Mode::Supervisor, irq);
    plic.set_threshold(hart, Mode::Supervisor, 0);
    log::info!("Register irq {} to hart {}", irq, hart);
}

pub fn init_plic(node: &FdtNode, info: &mut DtbInfo) -> bool {
    if node.name.starts_with(platform::PLIC_NAME) {
        let reg = node.reg().unwrap();
        reg.for_each(|x| info.arch.plic = x.starting_address as usize);
        true
    } else {
        false
    }
}
