extern crate alloc;

use arch::{Arch, ArchAsm};
use config::arch::CPU_NUM;
use ksync::Once;
use plic::{Mode, PLIC};

pub static PLIC: Once<PLIC<CPU_NUM>> = Once::new();

pub fn register_to_hart() {
    let plic = PLIC.get().unwrap();
    let hart = Arch::get_hartid();
    // todo: support multiple devices
    let irq = 1;
    plic.set_threshold(hart as u32, Mode::Machine, 1);
    plic.set_threshold(hart as u32, Mode::Supervisor, 0);
    plic.enable(hart as u32, Mode::Supervisor, irq);
    plic.complete(hart as u32, Mode::Supervisor, irq);
    log::debug!("Register irq {} to hart {}", irq, hart);
}

pub fn init_plic(plic_addr: usize) {
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
}
