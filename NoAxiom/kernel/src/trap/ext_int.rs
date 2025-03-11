use arch::{Arch, ArchInt};
use plic::Mode;

use crate::{
    config::fs::WAKE_NUM, cpu::get_hartid, driver::async_virtio_driver::virtio_mm::VIRTIO_BLOCK,
    platform::plic::PLIC,
};

pub fn ext_int_handler() {
    #[cfg(feature = "async_fs")]
    {
        let plic = PLIC.get().unwrap();
        let irq = plic.claim(get_hartid() as u32, Mode::Supervisor);
        // debug!("[SupervisorExternal] hart: {}, irq: {}", get_hartid(), irq);
        unsafe {
            VIRTIO_BLOCK
                .0
                .handle_interrupt()
                .expect("virtio handle interrupt error!");
            assert!(!Arch::is_interrupt_enabled());
            // debug!("virtio handle interrupt done!  Notify begin...");
            VIRTIO_BLOCK.0.wake_ops.notify(WAKE_NUM);
        };
        // debug!("Notify done!");
        plic.complete(get_hartid() as u32, Mode::Supervisor, irq);
        // debug!("plic complete done!");
    }
    #[cfg(not(feature = "async_fs"))]
    {
        let scause = Arch::read_trap_cause(); // scause::read();
        let stval = Arch::read_trap_value(); // stval::read();
        let sepc = Arch::read_trap_pc(); // sepc::read();
        panic!(
            "hart: {}, kernel SupervisorExternal interrupt is unsupported, stval = {:#x}, sepc = {:#x}",
            get_hartid(),
            stval,
            sepc
        )
    }
}
