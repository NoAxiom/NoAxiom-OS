mod blockcache;
pub mod blockdevice;
pub mod fat32;
mod file;
mod inode;
pub mod tmp;

use alloc::sync::Arc;
use core::mem::MaybeUninit;

use blockdevice::BlockDevice;
use fat32::FAT32FIleSystem;
pub use file::*;
use spin::Mutex;
pub use tmp::*;

use crate::{
    config::mm::VIRTIO0,
    driver::async_virtio_driver::{block::VirtIOBlock, mmio::VirtIOHeader},
    println,
};

// #[cfg(feature = "board_qemu")]
type BlockDeviceImpl = VirtIOBlock<1>;

#[cfg(feature = "board_k210")]
type BlockDeviceImpl = sdcard::SDCardWrapper;

lazy_static::lazy_static! {
    pub static ref VIRTIO_BLOCK: Arc<BlockDeviceImpl> = {
        let header = unsafe { &mut *(VIRTIO0 as *mut VirtIOHeader) };
        Arc::new(VirtIOBlock::new(header).unwrap())
    };
}

lazy_static::lazy_static! {
    // todo: async mutex?
    pub static ref FS: Arc<Mutex<MaybeUninit<FAT32FIleSystem>>> = Arc::new(Mutex::new(MaybeUninit::uninit()));
}

pub async fn fs_init() {
    let initialed_fs = FAT32FIleSystem::init(VIRTIO_BLOCK.clone() as Arc<dyn BlockDevice>).await;
    let mut gaurd = FS.lock();
    let ptr = gaurd.as_mut_ptr();
    unsafe {
        ptr.write(initialed_fs);
    }
    println!("[kernel] fs initialed");
}
