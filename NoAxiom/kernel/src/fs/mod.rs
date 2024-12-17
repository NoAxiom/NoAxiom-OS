mod blockcache;
pub mod blockdevice;
pub mod fat32;
mod file;
mod inode;
pub mod tmp;

use alloc::sync::Arc;
use core::mem::MaybeUninit;

use fat32::FAT32FIleSystem;
pub use file::*;
use kernel_sync::SpinMutex;
pub use tmp::*;

use crate::{
    arch::interrupt::{
        disable_external_interrupt, disable_global_interrupt, enable_external_interrupt,
        enable_global_interrupt,
    },
    driver::async_virtio_driver::virtio_mm::async_blk::VirtIOAsyncBlock,
};

#[cfg(feature = "riscv_qemu")]
type BlockDeviceImpl = VirtIOAsyncBlock;

#[cfg(feature = "board_k210")]
type BlockDeviceImpl = sdcard::SDCardWrapper;

lazy_static::lazy_static! {
    pub static ref VIRTIO_BLOCK: Arc<BlockDeviceImpl> = Arc::new(VirtIOAsyncBlock::new());
}

lazy_static::lazy_static! {
    // todo: async mutex?
    pub static ref FS: Arc<SpinMutex<MaybeUninit<FAT32FIleSystem>>> = Arc::new(SpinMutex::new(MaybeUninit::uninit()));
}

pub async fn fs_init() {
    info!("fs_init");
    enable_global_interrupt();
    enable_external_interrupt();
    let device = Arc::clone(&VIRTIO_BLOCK);
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
