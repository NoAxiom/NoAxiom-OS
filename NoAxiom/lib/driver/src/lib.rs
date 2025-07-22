#![no_std]
#![allow(deprecated)]
#![feature(impl_trait_in_assoc_type)]

use alloc::sync::Arc;

use ksync::{assert_no_lock, Once};

use crate::{
    archs::arch_driver_init,
    bus::probe_bus,
    devices::{block::BlockDevice, gpu::DisplayDevice, net::NetWorkDevice},
};
extern crate alloc;

mod archs;
mod bus;
pub mod devices;
mod irq;

pub use irq::handle_irq;

pub fn driver_init() {
    probe_bus();
    arch_driver_init();
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
