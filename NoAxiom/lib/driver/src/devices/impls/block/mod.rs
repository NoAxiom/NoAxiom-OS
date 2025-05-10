use alloc::boxed::Box;

use config::fs::BLOCK_SIZE;

use super::device::Device;
pub mod async_virtio_driver;
pub mod virtio_block;

#[async_trait::async_trait]
#[allow(unused_variables)]
pub trait BlockDevice: Device {
    async fn read_block(&self, id: usize, buf: &mut [u8; BLOCK_SIZE]) {
        unimplemented!("{} not implement read_block!", self.device_name())
    }

    async fn write_block(&self, id: usize, buf: &[u8; BLOCK_SIZE]) {
        unimplemented!("{} not implement write_block!", self.device_name())
    }
}
