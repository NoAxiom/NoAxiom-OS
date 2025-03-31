// ignore warnings for this module
// #![allow(warnings)]

mod blockcache;
pub mod fat32;
pub mod fdtable;
pub mod manager;
pub mod path;
pub mod pipe;
pub mod stdio;
pub mod vfs;

use arch::{Arch, ArchInt};

pub async fn fs_init() {
    let interrupt = Arch::is_interrupt_enabled();
    let extertnel_interrupt = Arch::is_external_interrupt_enabled();
    info!(
        "[fs] interrupt: {}, external interrupt: {}",
        interrupt, extertnel_interrupt
    );
    Arch::enable_interrupt();
    Arch::enable_external_interrupt();
    vfs::fs_init().await;
    info!("[fs] fs init done");
    if !interrupt {
        // info!("disable global interrupt");
        // interrupt::disable_interrupt();
    }
    if !extertnel_interrupt {
        // info!("disable external interrupt");
        // interrupt::disable_external_interrupt();
    }
}
