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
use driver::manager::DEV_BUS;
use kfuture::block::block_on;

pub async fn fs_init() {
    info!("[fs] fs_init start");
    Arch::enable_interrupt();
    info!(
        "[fs] interrupt: {}, external interrupt: {}",
        Arch::is_interrupt_enabled(),
        Arch::is_external_interrupt_enabled()
    );
    vfs::vfs_init().await;
}

#[allow(unused)]
pub fn test() {
    use blockcache::get_block_cache;
    // crate::sched::utils::block_on(driver::blk_dev_test(1000, 100000));
    // let dev = get_block_cache();
    let dev = DEV_BUS.get_default_block_device().unwrap();

    vfs::impls::ext4::ext4_rs_test(dev);
}

pub fn disk_sync() {
    println_debug!("[kernel] begin sync to the disk!");
    pagecache::get_pagecache_wguard().sync_all();
    block_on(blockcache::get_block_cache().sync_all()).expect("[kernel] sync block cache failed!");
    println_debug!("[kernel] sync to the disk succeed!");
}
