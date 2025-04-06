use alloc::sync::Arc;

use basic::dentry::Dentry;
use driver::devices::impls::block::BlockDevice;
use impls::{ext4::filesystem::AsyncSmpExt4, rust_fat32::filesystem::AsyncSmpFat32};
use ksync::Once;

use crate::{config::fs::BLOCK_SIZE, fs::manager::FS_MANAGER, include::fs::MountFlags};
pub mod basic;
mod impls;

// type RealFs = FAT32FIleSystem;
type RealFs2 = AsyncSmpFat32;
type RealFs = AsyncSmpExt4;

lazy_static::lazy_static! {
    static ref ROOT_DENTRY: Once<Arc<dyn Dentry>> = Once::new();
}

pub fn chosen_device() -> Arc<&'static dyn BlockDevice> {
    driver::get_blk_dev()
}

pub async fn device_test(device: Arc<&'static dyn BlockDevice>) {
    let mut read_buf = [0u8; BLOCK_SIZE];
    for i in 0..4 {
        device
            .read(i as usize, &mut read_buf)
            .await
            .expect("Blk dev read failed");
    }
    info!("Block Device works well!");
}

/// Create the root dentry, mount multiple fs
pub async fn fs_init() {
    info!("[vfs] fs initial, register file systems");
    let fs_name = RealFs::name();
    FS_MANAGER.register(Arc::new(RealFs::new(fs_name)));
    FS_MANAGER.register(Arc::new(RealFs::new(RealFs2::name())));
    // todo: virtual fs support

    info!("[vfs] fs initial, mounting the inital real fs: {}", fs_name);
    let device = chosen_device();
    device_test(device.clone()).await;

    let disk_fs = FS_MANAGER.get(fs_name).unwrap();
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
