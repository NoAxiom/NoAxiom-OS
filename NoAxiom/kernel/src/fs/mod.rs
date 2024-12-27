mod blockcache;
pub mod fat32;
mod file;
pub mod inode;
pub mod path;

use alloc::sync::Arc;
use core::mem::MaybeUninit;

use fat32::FAT32FIleSystem;
pub use file::*;
use kernel_sync::SpinMutex;

// pub use tmp::*;
use crate::arch::interrupt::disable_global_interrupt;
#[cfg(not(feature = "async_fs"))]
use crate::device::block::BLOCK_DEVICE as SYNC_BLOCK_DEVICE;
#[cfg(feature = "async_fs")]
use crate::driver::async_virtio_driver::virtio_mm::VIRTIO_BLOCK;

lazy_static::lazy_static! {
    // todo: async mutex?
    pub static ref FS: Arc<SpinMutex<MaybeUninit<FAT32FIleSystem>>> = Arc::new(SpinMutex::new(MaybeUninit::uninit()));
}

pub async fn fs_init() {
    let device;
    #[cfg(feature = "async_fs")]
    {
        info!("async_fs init");
        enable_global_interrupt();
        enable_external_interrupt();
        device = Arc::clone(&VIRTIO_BLOCK);
    }
    #[cfg(not(feature = "async_fs"))]
    {
        info!("sync_fs init");
        device = Arc::clone(SYNC_BLOCK_DEVICE.get().unwrap());
    }
    let initialed_fs = FAT32FIleSystem::init(device).await;
    info!("initialed_fs done");
    initialed_fs.list().await;
    let mut guard = FS.lock();
    let ptr = guard.as_mut_ptr();
    unsafe {
        ptr.write(initialed_fs);
    }
    drop(guard);
    disable_global_interrupt();
    info!("[kernel] fs initialed");
}
