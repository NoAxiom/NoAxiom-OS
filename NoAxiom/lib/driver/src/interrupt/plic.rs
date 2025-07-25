use arch::{consts::IO_ADDR_OFFSET, Arch, ArchAsm, ArchInt};
use config::cpu::CPU_NUM;
use log::debug;
use plic::{Mode, PLIC};

use crate::{basic::Device, block::BlockDevice, get_blk_dev, interrupt::InterruptDevice};

pub struct PlicDevice {
    dev: PLIC<CPU_NUM>,
}

impl Device for PlicDevice {
    fn device_name(&self) -> &'static str {
        "RISC-V PLIC Device"
    }
    fn device_type(&self) -> &'static crate::basic::DeviceType {
        &crate::basic::DeviceType::Unknown
    }
}

impl InterruptDevice for PlicDevice {
    fn handle_irq(&self) {
        if let Some(blk) = get_blk_dev() {
            self.handle_irq_with_blk(blk)
        }
    }
}

impl PlicDevice {
    pub fn new(addr: usize) -> Self {
        let dev = Self::new_plic(addr);
        dev.register_to_all_harts();
        dev.disable_blk_irq();
        #[cfg(feature = "intable")]
        {
            dev.enable_blk_irq();
        }
        dev
    }

    fn inner(&self) -> &PLIC<CPU_NUM> {
        &self.dev
    }

    fn handle_irq_with_blk(&self, blk: &'static dyn BlockDevice) {
        assert!(!Arch::is_interrupt_enabled());
        let irq = self.claim();
        log::error!("[driver] handle irq: {}", irq);
        if irq == 1 {
            blk.handle_interrupt().expect("handle interrupt error");
        } else {
            log::error!("[driver] unhandled irq: {}", irq);
        }
        self.complete(irq);
        log::error!("[driver] handle irq: {} finished", irq);
        assert!(!Arch::is_interrupt_enabled());
    }

    fn disable_blk_irq(&self) {
        let irq = 1;
        let hart = Arch::get_hartid();
        self.inner().disable(hart as u32, Mode::Supervisor, irq);
    }

    fn enable_blk_irq(&self) {
        let irq = 1;
        let hart = Arch::get_hartid();
        self.inner().enable(hart as u32, Mode::Supervisor, irq);
    }

    fn claim(&self) -> u32 {
        let hart = Arch::get_hartid();
        self.inner().claim(hart as u32, Mode::Supervisor)
    }

    fn complete(&self, irq: u32) {
        let hart = Arch::get_hartid();
        self.inner().complete(hart as u32, Mode::Supervisor, irq);
    }

    fn new_plic(plic_addr: usize) -> Self {
        let plic_addr = plic_addr | IO_ADDR_OFFSET;
        debug!("PLIC addr: {:#x}", plic_addr);
        let privileges = [2; CPU_NUM];
        let plic = PLIC::new(plic_addr, privileges);

        let priority = 1;
        let irq = 1;
        plic.set_priority(irq, priority);

        // todo: register more devices
        log::info!("Init plic success");
        #[cfg(any(feature = "vf2"))]
        {
            panic!();
            let mut privileges = [2; CPU_NUM];
            // core 0 don't have S mode
            privileges[0] = 1;
            log::debug!("PLIC context: {:?}", privileges);
            let plic = PLIC::new(plic_addr, privileges);
            PLIC.call_once(|| plic);
            log::debug!("Init hifive or vf2 plic success");
        }

        Self { dev: plic }
    }

    fn register_to_hart(&self, hart: u32) {
        let plic = &self.dev;
        let irq = 1;
        plic.enable(hart, Mode::Supervisor, irq);
        plic.set_threshold(hart, Mode::Supervisor, 0);
        log::info!("Register irq {} to hart {}", irq, hart);
    }

    fn register_to_all_harts(&self) {
        for i in 0..CPU_NUM {
            self.register_to_hart(i as u32);
        }
    }
}
