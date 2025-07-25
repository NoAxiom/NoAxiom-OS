// ignore warnings for this module
// #![allow(warnings)]

pub mod blockcache;
pub mod fdtable;
pub mod manager;
pub mod pagecache;
pub mod path;
pub mod pipe;
pub mod vfs;

use arch::{Arch, ArchInt};
use device::DEV_BUS;

pub async fn init() {
    info!(
        "[fs] interrupt: {}, external interrupt: {}",
        Arch::is_interrupt_enabled(),
        Arch::is_external_interrupt_enabled()
    );
    vfs::fs_init().await;
}

#[allow(unused)]
pub fn test() {
    use blockcache::get_block_cache;
    // crate::sched::utils::block_on(driver::blk_dev_test(1000, 100000));
    // let dev = get_block_cache();
    let dev = DEV_BUS.get_default_block_device().unwrap();

    vfs::impls::ext4::ext4_rs_test(dev);
}
