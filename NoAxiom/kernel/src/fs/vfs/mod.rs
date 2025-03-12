use alloc::sync::Arc;

use basic::dentry::Dentry;
use impls::ext4::filesystem::AsyncSmpExt4;
use ksync::Once;

use crate::{
    config::fs::BLOCK_SIZE, device::block::BlockDevice, fs::manager::FS_MANAGER,
    include::fs::MountFlags,
};
pub mod basic;
mod impls;

// type RealFs = FAT32FIleSystem;
// type RealFs = AsyncSmpFat32;
type RealFs = AsyncSmpExt4;

lazy_static::lazy_static! {
    static ref ROOT_DENTRY: Once<Arc<dyn Dentry>> = Once::new();
}

pub fn chosen_device() -> Arc<dyn BlockDevice> {
    let device;
    #[cfg(feature = "async_fs")]
    {
        use crate::driver::async_virtio_driver::virtio_mm::VIRTIO_BLOCK;

        info!("async_fs init");
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

pub async fn device_test(device: Arc<dyn BlockDevice>) {
    let mut read_buf = [0u8; BLOCK_SIZE];
    for i in 0..4 {
        device.read(i as usize, &mut read_buf).await;
    }
    info!("Block Device works well!");
}

/// Create the root dentry, mount multiple fs
pub async fn fs_init() {
    info!("[vfs] fs initial, register file systems");
    FS_MANAGER.register(Arc::new(RealFs::new("vfat")));
    // todo: virtual fs support

    info!("[vfs] fs initial, mounting the inital real fs");
    let device = chosen_device();
    device_test(device.clone()).await;
    let disk_fs = FS_MANAGER.get("vfat").unwrap();
    let root = disk_fs
        .root(None, MountFlags::empty(), "/", Some(device))
        .await; // the root also the vfs root
    ROOT_DENTRY.call_once(|| root);

    // Load the root dentry
    root_dentry().open().unwrap().load_dir().await.unwrap();
}

pub fn root_dentry() -> Arc<dyn basic::dentry::Dentry> {
    ROOT_DENTRY.get().unwrap().clone()
}
