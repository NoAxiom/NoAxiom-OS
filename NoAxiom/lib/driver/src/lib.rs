#![no_std]
#![allow(deprecated)]
#![feature(impl_trait_in_assoc_type)]

use alloc::sync::Arc;

use ksync::{assert_no_lock, Once};

use crate::devices::{block::BlockDevice, gpu::DisplayDevice, net::NetWorkDevice};

mod bus;
pub mod devices;
mod dtb;
#[cfg(target_arch = "riscv64")]
mod plic;

extern crate alloc;

pub fn init(dtb: usize) {
    let dtb = dtb | arch::consts::KERNEL_ADDR_OFFSET;
    log::debug!("[driver] init with dtb: {:#x}", dtb);
    dtb::init(dtb);
    bus::probe_bus();

    // the plic is used only for riscv64 arch
    #[cfg(target_arch = "riscv64")]
    {
        plic::init();
        plic::disable_blk_irq();
        #[cfg(feature = "intable")]
        {
            plic::enable_blk_irq();
        }
    }
}

lazy_static::lazy_static! {
    pub static ref BLK_DEV: Once<Arc<&'static dyn BlockDevice>> = Once::new();
    pub static ref NET_DEV: Once<Arc<&'static dyn NetWorkDevice>> = Once::new();
    pub static ref DISPLAY_DEV: Once<Arc<&'static dyn DisplayDevice>> = Once::new();
}

pub fn get_blk_dev() -> Arc<&'static dyn BlockDevice> {
    Arc::clone(BLK_DEV.get().unwrap())
}

pub fn get_net_dev() -> Arc<&'static dyn NetWorkDevice> {
    Arc::clone(NET_DEV.get().unwrap())
}

pub fn get_display_dev() -> Arc<&'static dyn DisplayDevice> {
    Arc::clone(DISPLAY_DEV.get().unwrap())
}

#[cfg(target_arch = "riscv64")]
pub fn handle_irq() {
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

#[cfg(target_arch = "loongarch64")]
pub fn handle_irq() {
    unimplemented!("LoongArch64 does not support IRQ handling yet");
}

/// just for test blk_dev and return `!`
#[allow(unused)]
pub async fn blk_dev_test(start: usize, len: usize) -> ! {
    use alloc::vec;
    let device = get_blk_dev();

    log::debug!("[driver] test block device");
    let write_buf = vec![0x41 as u8; 512];
    assert_no_lock!();
    for sector in start..start + len {
        device.write(sector, &write_buf).await.unwrap();
    }
    for sector in start..start + len {
        let mut read_buf = vec![0u8; 512];
        device.read(sector, &mut read_buf).await.unwrap();
        if read_buf != write_buf {
            panic!(
                "read and write failed at {}: {} != {}",
                sector, read_buf[0], write_buf[0]
            );
        }
    }
    panic!("[driver] test block device success")
}
