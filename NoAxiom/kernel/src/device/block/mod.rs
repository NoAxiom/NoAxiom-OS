pub mod memoryimg;
pub mod vf2sd;
pub mod virtio;
use alloc::sync::Arc;

use spin::Once;

use super::Device;
use crate::{utils::result::Errno, device::block::virtio::virtio as Virtio, println};

/// ! fixme: Virtio to dyn BlockDevice
pub static BLOCK_DEVICE: Once<Arc<Virtio>> = Once::new();

pub fn init_block_device(block_device: Arc<Virtio>) {
    BLOCK_DEVICE.call_once(|| block_device);
}
pub trait BlockDevice: Device {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> Result<(), Errno>;
    fn write_block(&self, block_id: usize, buf: &[u8]) -> Result<(), Errno>;
    fn read_async_block(&self, block_id: usize, buf: &mut [u8]) -> Result<(), Errno>;
    fn write_async_block(&self, block_id: usize, buf: &[u8]) -> Result<(), Errno>;
    fn size(&self) -> usize;
    fn flush(&self) -> Result<(), Errno>;
}
impl dyn BlockDevice {
    pub fn read(&self, buf: &mut [u8], offset: usize) -> Result<(), Errno> {
        let blk = offset / 4096;
        println!("block device read");
        self.read_block(blk, buf)?;
        Ok(())
    }
    pub fn write(&self, buf: &[u8], offset: usize) -> Result<(), Errno> {
        let blk = offset / 4096;
        self.write_block(blk, buf)?;
        Ok(())
    }
    pub fn read_async(&self, buf: &mut [u8], offset: usize) -> Result<(), Errno> {
        let blk = offset % 4096;
        self.read_async_block(blk, buf)?;
        Ok(())
    }
    pub fn write_async(&self, buf: &[u8], offset: usize) -> Result<(), Errno> {
        let blk = offset % 4096;
        self.write_async_block(blk, buf)?;
        Ok(())
    }
}
