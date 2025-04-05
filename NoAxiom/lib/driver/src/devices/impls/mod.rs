pub mod block;
pub mod device;
pub mod gpu;
pub mod net;
pub mod virtio;

cfg_if::cfg_if! {
    if #[cfg(feature = "async_fs")] {
        use crate::devices::impls::block::async_virtio_driver::virtio_mm::async_blk::VirtIOAsyncBlock;
        pub type BlkDevice = VirtIOAsyncBlock;
    } else {
        use block::virtio_block::VirtioBlockType;
        pub type BlkDevice = VirtioBlockType;
    }
}

use net::loopback::LoopBackDev;
pub type NetDevice = LoopBackDev;

use gpu::virtio_gpu::VirtioGpu;
pub type DisplayDevice = VirtioGpu;
