use super::device::Device;

pub mod async_virtio_driver;
pub mod virtio_block;

#[async_trait::async_trait]
pub trait BlockDevice: Device {}
