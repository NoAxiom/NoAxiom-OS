#![no_std]
#![allow(deprecated)]
#![feature(core_intrinsics)]

use alloc::sync::Arc;

use devices::{
    impls::{device::BlockDevice, net::NetWorkDev},
    ALL_DEVICES,
};
use ksync::assert_no_lock;

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
    plic::init();
}

pub fn get_blk_dev() -> Arc<&'static dyn BlockDevice> {
    let blk = ALL_DEVICES.as_ref().get_blk_device().unwrap();
    Arc::new(blk)
}

pub fn get_net_dev() -> Arc<&'static dyn NetWorkDev> {
    let net = ALL_DEVICES.as_ref().get_net_device().unwrap();
    Arc::new(net)
}

pub fn get_display_dev() -> Arc<&'static devices::impls::DisplayDevice> {
    let display = ALL_DEVICES.as_ref().get_display_device().unwrap();
    Arc::new(display)
}

pub fn handle_irq() {
    #[cfg(any(feature = "interruptable_async", feature = "full_func"))]
    {
        use arch::{Arch, ArchInt};
        assert!(!Arch::is_interrupt_enabled());
        let irq = plic::claim();
        if irq == 1 {
            get_blk_dev()
                .handle_interrupt()
                .expect("handle interrupt error");
        } else {
            log::error!("[driver] unhandled irq: {}", irq);
        }
        plic::complete(irq);
        assert!(!Arch::is_interrupt_enabled());
    }
    #[cfg(feature = "async")]
    {
        unreachable!("sync fs shouldn't accept interrupt!");
    }
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
