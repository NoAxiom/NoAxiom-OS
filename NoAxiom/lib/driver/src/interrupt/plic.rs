use arch::{consts::IO_ADDR_OFFSET, Arch, ArchAsm, ArchInt};
use array_init::array_init;
use config::cpu::{CPU_NUM, PLIC_SLOTS};
use ksync::Once;
use log::debug;
use plic::{Mode, PLIC};

use crate::{
    basic::{DevResult, Device},
    interrupt::{InterruptControllerDevice, InterruptDevice},
};

pub struct PlicDevice {
    controller: PLIC<CPU_NUM>,
    devices: [[Once<&'static dyn InterruptDevice>; PLIC_SLOTS]; CPU_NUM],
}

impl Device for PlicDevice {
    fn device_name(&self) -> &'static str {
        "RISC-V PLIC Device"
    }
    fn device_type(&self) -> &'static crate::basic::DeviceType {
        &crate::basic::DeviceType::Interrupt(crate::basic::InterruptDeviceType::PLIC)
    }
}

impl InterruptDevice for PlicDevice {
    fn handle_irq(&self) -> DevResult<()> {
        let entry = &self.devices[Arch::get_hartid()][0];
        let dev = *entry.get().unwrap();
        self.handle_irq_with_dev(dev)
    }
}

impl InterruptControllerDevice for PlicDevice {
    fn register_dev(&self, dev: &'static dyn InterruptDevice) {
        log::info!("[driver] Registering device: {}", dev.device_name());
        // todo add hart here
        for i in 0..CPU_NUM {
            self.devices[i][0].call_once(|| dev);
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
        &self.controller
    }

    fn handle_irq_with_dev(&self, dev: &'static dyn InterruptDevice) -> DevResult<()> {
        assert!(!Arch::is_interrupt_enabled());
        let irq = self.claim();
        log::error!("[driver] handle irq: {}", irq);
        if irq == 1 {
            dev.handle_irq()?;
        } else {
            log::error!("[driver] unhandled irq: {}", irq);
        }
        self.complete(irq);
        log::error!("[driver] handle irq: {} finished", irq);
        assert!(!Arch::is_interrupt_enabled());
        Ok(())
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

        Self {
            controller: plic,
            devices: array_init(|_| array_init(|_| Once::new())),
        }
    }

    fn register_to_hart(&self, hart: u32) {
        let plic = &self.controller;
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
