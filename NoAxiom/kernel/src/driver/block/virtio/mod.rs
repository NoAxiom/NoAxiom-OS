//! 块设备驱动层
extern crate alloc;
pub mod virtio_blk;
pub mod virtio_gpu;
pub mod virtio_input;
use alloc::sync::Arc;

use lazy_static::lazy_static;
use virtio_gpu::{GpuDevice, VirtIOGpuWrapper};
use virtio_input::{InputDevice, VirtIOInputWrapper};

use crate::config::mm::KERNEL_ADDR_OFFSET;

mod virtio_impl;

// keyboard
const VIRTIO5: usize = 0x10005000 + KERNEL_ADDR_OFFSET;
// mouse
const VIRTIO6: usize = 0x10006000 + KERNEL_ADDR_OFFSET;

lazy_static! {
    pub static ref GPU_DEVICE: Arc<dyn GpuDevice> = Arc::new(VirtIOGpuWrapper::new());
}
lazy_static! {
    pub static ref KEYBOARD_DEVICE: Arc<dyn InputDevice> =
        Arc::new(VirtIOInputWrapper::new(VIRTIO5));
}
lazy_static! {
    pub static ref MOUSE_DEVICE: Arc<dyn InputDevice> = Arc::new(VirtIOInputWrapper::new(VIRTIO6));
}
