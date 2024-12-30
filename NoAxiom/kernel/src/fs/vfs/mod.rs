use alloc::sync::Arc;

use basic::{dentry::Dentry, filesystem::FileSystem};
use impls::fat32::filesystem::FAT32FIleSystem;
use spin::Once;

use crate::{
    arch::interrupt::disable_external_interrupt, device::block::BlockDevice, nix::fs::MountFlags,
};
pub mod basic;
mod impls;

type RealFs = FAT32FIleSystem;

lazy_static::lazy_static! {
    static ref ROOT_DENTRY: Once<Arc<dyn Dentry>> = Once::new();
}

pub fn chosen_device() -> Arc<dyn BlockDevice> {
    let device;
    #[cfg(feature = "async_fs")]
    {
        use crate::{
            arch::interrupt::{enable_external_interrupt, enable_global_interrupt},
            driver::async_virtio_driver::virtio_mm::VIRTIO_BLOCK,
        };
        info!("async_fs init");
        // enable_global_interrupt();
        enable_external_interrupt();
        device = Arc::clone(&VIRTIO_BLOCK);
    }
    #[cfg(not(feature = "async_fs"))]
    {
        use crate::device::block::BLOCK_DEVICE as SYNC_BLOCK_DEVICE;
        info!("[vfs] sync_fs init");
        device = Arc::clone(SYNC_BLOCK_DEVICE.get().unwrap());
    }
    device
}

/// Create the root dentry, mount multiple fs
pub async fn fs_init() {
    info!("[vfs] fs initial, mounting real fs");
    let device = chosen_device();
    let disk_fs = Arc::new(RealFs::new("FAT32"));
    let root = disk_fs
        .root(None, MountFlags::empty(), "/", Some(device))
        .await; // the root also the vfs root
    ROOT_DENTRY.call_once(|| root);

    // todo: virtual fs support

    // Load the root dentry
    root_dentry().open().unwrap().load_dir().await.unwrap();

    disable_external_interrupt();
}

pub fn root_dentry() -> Arc<dyn basic::dentry::Dentry> {
    ROOT_DENTRY.get().unwrap().clone()
}
