// ignore warnings for this module
// #![allow(warnings)]

mod blockcache;
pub mod fat32;
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
