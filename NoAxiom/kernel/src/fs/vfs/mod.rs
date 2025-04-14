use alloc::sync::Arc;

use basic::dentry::Dentry;
use driver::devices::impls::block::BlockDevice;
use impls::{
    ext4::filesystem::AsyncSmpExt4, proc::filesystem::ProcDevFs,
    rust_fat32::filesystem::AsyncSmpFat32,
};
use ksync::Once;

use crate::{config::fs::BLOCK_SIZE, fs::manager::FS_MANAGER, include::fs::MountFlags};
pub mod basic;
mod impls;

lazy_static::lazy_static! {
    static ref ROOT_DENTRY: Once<Arc<dyn Dentry>> = Once::new();
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
    type RootRealFs = AsyncSmpExt4;

    info!("[vfs] fs initial, register file systems");
    FS_MANAGER.register(Arc::new(AsyncSmpExt4::new(AsyncSmpExt4::name())));
    FS_MANAGER.register(Arc::new(AsyncSmpFat32::new(AsyncSmpFat32::name())));
    FS_MANAGER.register(Arc::new(ProcDevFs::new(ProcDevFs::name())));

    info!("[vfs] [{}] mounting", RootRealFs::name());
    let device = driver::get_blk_dev();
    device_test(device.clone()).await;

    let disk_fs = FS_MANAGER.get(RootRealFs::name()).unwrap();
    let root = disk_fs
        .root(None, MountFlags::empty(), "/", Some(device))
        .await; // the root also the vfs root
    ROOT_DENTRY.call_once(|| root.clone());

    let proc_fs = FS_MANAGER.get(ProcDevFs::name()).unwrap();
    let _proc_root = proc_fs
        .root(Some(root.clone()), MountFlags::empty(), "proc", None)
        .await;

    // Load the root dentry
    root_dentry()
        .open()
        .unwrap()
        .load_dir()
        .await
        .expect("load root dir failed");
}

pub fn root_dentry() -> Arc<dyn basic::dentry::Dentry> {
    ROOT_DENTRY.get().unwrap().clone()
}
