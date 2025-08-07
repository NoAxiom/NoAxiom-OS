use alloc::sync::Arc;

use basic::dentry::Dentry;
use impls::{
    devfs::filesystem::DevFs, ext4::filesystem::AsyncSmpExt4, proc::filesystem::ProcDevFs,
    rust_fat32::filesystem::AsyncSmpFat32,
};
use ksync::Once;

use crate::{
    fs::{blockcache::get_block_cache, manager::FS_MANAGER, path::kcreate},
    include::fs::{FileFlags, InodeMode, MountFlags, ALL_PERMISSIONS_MASK},
};
pub mod basic;
pub mod impls;

pub use impls::{devfs::TTYFILE, inc_interrupts_count};

lazy_static::lazy_static! {
    static ref ROOT_DENTRY: Once<Arc<dyn Dentry>> = Once::new();
}

/// Create the root dentry, mount multiple fs
pub async fn vfs_init() {
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
        .open(&FileFlags::empty())
        .unwrap()
        .load_dir()
        .await
        .expect("load root dir failed");

    // let passwd = Path::from_or_create(String::from("/etc/passwd"),
    // InodeMode::FILE).await; passwd.dentry().open().expect("open /etc/passwd
    // failed");

    info!("[fs] create /bin");
    kcreate(
        "/bin",
        InodeMode::DIR | InodeMode::from_bits(ALL_PERMISSIONS_MASK).unwrap(),
    );

    #[cfg(feature = "debug_sig")]
    {
        use crate::fs::path::kcreate;

        let logon = kcreate(
            "/logon",
            InodeMode::FILE | InodeMode::from_bits(ALL_PERMISSIONS_MASK).unwrap(),
        );
        logon.open(&FileFlags::empty()).expect("open logon failed");

        let logoff = kcreate(
            "/logoff",
            InodeMode::FILE | InodeMode::from_bits(ALL_PERMISSIONS_MASK).unwrap(),
        );
        logoff
            .open(&FileFlags::empty())
            .expect("open logoff failed");
    }
}

pub fn root_dentry() -> Arc<dyn basic::dentry::Dentry> {
    ROOT_DENTRY.get().unwrap().clone()
}
