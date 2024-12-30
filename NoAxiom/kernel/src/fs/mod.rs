// ignore warnings for this module
#![allow(warnings)]

mod blockcache;
pub mod fat32;
pub mod path;
pub mod vfs;

use alloc::sync::Arc;

use vfs::basic::dentry::Dentry;

pub async fn fs_init() {
    vfs::fs_init().await;
}

pub fn fs_root() -> Arc<dyn Dentry> {
    vfs::root_dentry()
}
