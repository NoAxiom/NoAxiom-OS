// ignore warnings for this module
// #![allow(warnings)]

mod blockcache;
pub mod fat32;
pub mod fdtable;
pub mod path;
pub mod pipe;
pub mod stdio;
pub mod vfs;

use alloc::sync::Arc;

use arch::interrupt::{self, enable_external_interrupt, enable_global_interrupt};
use vfs::basic::dentry::Dentry;

pub async fn fs_init() {
    let interrupt = interrupt::is_interrupt_enabled();
    let extertnel_interrupt = interrupt::is_external_interrupt_enabled();
    info!(
        "[fs] interrupt: {}, external interrupt: {}",
        interrupt, extertnel_interrupt
    );
    enable_global_interrupt();
    enable_external_interrupt();
    vfs::fs_init().await;
    if !interrupt {
        // info!("disable global interrupt");
        // interrupt::disable_global_interrupt();
    }
    if !extertnel_interrupt {
        // info!("disable external interrupt");
        // interrupt::disable_external_interrupt();
    }
}

pub fn fs_root() -> Arc<dyn Dentry> {
    vfs::root_dentry()
}
