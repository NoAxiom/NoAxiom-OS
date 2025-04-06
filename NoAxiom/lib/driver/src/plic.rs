extern crate alloc;

use arch::{Arch, ArchAsm, ArchMemory};
use config::cpu::CPU_NUM;
use ksync::Once;
use log::debug;
use plic::{Mode, PLIC};

use crate::dtb::dtb_info;

pub static PLIC: Once<PLIC<CPU_NUM>> = Once::new();

pub fn claim() -> u32 {
    let plic = PLIC.get().unwrap();
    let hart = Arch::get_hartid();
    plic.claim(hart as u32, Mode::Supervisor)
}

pub fn complete(irq: u32) {
    let plic = PLIC.get().unwrap();
    let hart = Arch::get_hartid();
    plic.complete(hart as u32, Mode::Supervisor, irq);
}

pub fn init() {
    let plic_addr = dtb_info().plic | arch::Arch::KERNEL_ADDR_OFFSET;
    debug!("PLIC addr: {:#x}", plic_addr);
    let privileges = [2; CPU_NUM];
    let plic = PLIC::new(plic_addr, privileges);
    PLIC.call_once(|| plic);

    let priority;
    #[cfg(feature = "async_fs")]
    {
        priority = 2;
    }
    #[cfg(not(feature = "async_fs"))]
    {
        priority = 0;
    }
    // ! fixme: now is turn OFF the interrupt
    let irq = 1;
    let plic = PLIC.get().unwrap();
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

    register_to_hart();
}

pub fn register_to_hart() {
    let plic = PLIC.get().unwrap();
    let hart = Arch::get_hartid();
    // todo: support multiple devices
    let irq = 1;
    plic.set_threshold(hart as u32, Mode::Machine, 1);
    plic.set_threshold(hart as u32, Mode::Supervisor, 0);
    plic.enable(hart as u32, Mode::Supervisor, irq);
    plic.complete(hart as u32, Mode::Supervisor, irq);
    log::info!("Register irq {} to hart {}", irq, hart);
}
