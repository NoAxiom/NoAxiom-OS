// ignore warnings for this module
// #![allow(warnings)]

pub mod blockcache;
pub mod fdtable;
pub mod manager;
pub mod path;
pub mod pipe;
pub mod vfs;

use arch::{Arch, ArchInt};

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
    use driver::get_blk_dev;
    // crate::sched::utils::block_on(driver::blk_dev_test(1000, 100000));
    // let dev = get_block_cache();
    let dev = get_blk_dev();

    vfs::impls::ext4::ext4_rs_test(dev);
}
