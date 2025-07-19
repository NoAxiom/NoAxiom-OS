use alloc::sync::Arc;

use basic::dentry::Dentry;
use impls::{
    devfs::filesystem::DevFs, ext4::filesystem::AsyncSmpExt4, proc::filesystem::ProcDevFs,
    rust_fat32::filesystem::AsyncSmpFat32,
};
use ksync::Once;

use crate::{
    fs::{blockcache::get_block_cache, manager::FS_MANAGER, path::Path},
    include::fs::{InodeMode, MountFlags},
};
pub mod basic;
pub mod impls;

pub use impls::{devfs::TTYFILE, inc_interrupts_count};

lazy_static::lazy_static! {
    static ref ROOT_DENTRY: Once<Arc<dyn Dentry>> = Once::new();
}

/// Create the root dentry, mount multiple fs
pub async fn fs_init() {
    type RootRealFs = AsyncSmpExt4;

    info!("[vfs] fs initial, register file systems");
    FS_MANAGER.register(Arc::new(AsyncSmpExt4::new(AsyncSmpExt4::name())));
    FS_MANAGER.register(Arc::new(AsyncSmpFat32::new(AsyncSmpFat32::name())));
    FS_MANAGER.register(Arc::new(ProcDevFs::new(ProcDevFs::name())));
    FS_MANAGER.register(Arc::new(DevFs::new(DevFs::name())));

    info!("[vfs] [{}] mounting", RootRealFs::name());
    let device = get_block_cache();

    let disk_fs = FS_MANAGER.get(RootRealFs::name()).unwrap();
    let root = disk_fs
        .root(None, MountFlags::empty(), "/", Some(device))
        .await; // the root also the vfs root
    ROOT_DENTRY.call_once(|| root.clone());

    let proc_fs = FS_MANAGER.get(ProcDevFs::name()).unwrap();
    let _proc_root = proc_fs
        .root(Some(root.clone()), MountFlags::empty(), "proc", None)
        .await;

    let dev_fs = FS_MANAGER.get(DevFs::name()).unwrap();
    let _dev_root = dev_fs
        .root(Some(root.clone()), MountFlags::empty(), "dev", None)
        .await;

    // Load the root dentry
    root_dentry()
        .open()
        .unwrap()
        .load_dir()
        .await
        .expect("load root dir failed");

    // let passwd = Path::from_or_create(String::from("/etc/passwd"),
    // InodeMode::FILE).await; passwd.dentry().open().expect("open /etc/passwd
    // failed");

    #[cfg(feature = "debug_sig")]
    {
        let ls = Path::from_or_create(format!("/ls"), InodeMode::FILE)
            .await
            .unwrap();
        ls.dentry().open().expect("open ls failed");

        let logon = Path::from_or_create(format!("/logon"), InodeMode::FILE)
            .await
            .unwrap();
        logon.dentry().open().expect("open logon failed");

        let logoff = Path::from_or_create(format!("/logoff"), InodeMode::FILE)
            .await
            .unwrap();
        logoff.dentry().open().expect("open logoff failed");
    }
}

pub fn root_dentry() -> Arc<dyn basic::dentry::Dentry> {
    ROOT_DENTRY.get().unwrap().clone()
}
