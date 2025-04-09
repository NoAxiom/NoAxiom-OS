use alloc::boxed::Box;

use spin::Mutex;
use virtio_drivers_async::{
    device::blk::VirtIOBlk,
    transport::{mmio::MmioTransport, pci::PciTransport},
};

use super::BlockDevice;
use crate::devices::impls::{
    device::{DevResult, Device},
    virtio::{dev_err, VirtioHalImpl},
};

pub enum VirtioBlockType {
    Pci(Mutex<VirtIOBlk<VirtioHalImpl, PciTransport>>),
    Mmio(Mutex<VirtIOBlk<VirtioHalImpl, MmioTransport>>),
}

#[async_trait::async_trait]
impl Device for VirtioBlockType {
    fn device_name(&self) -> &'static str {
        match self {
            VirtioBlockType::Pci(_) => "virtio_block_pci",
            VirtioBlockType::Mmio(_) => "virtio_block_mmio",
        }
    }
    async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        match self {
            VirtioBlockType::Pci(blk) => blk.lock().read_blocks(id, buf).await.map_err(dev_err)?,
            VirtioBlockType::Mmio(blk) => blk.lock().read_blocks(id, buf).await.map_err(dev_err)?,
        }
        Ok(buf.len())
    }
    async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        match self {
            VirtioBlockType::Pci(blk) => blk.lock().write_blocks(id, buf).await.map_err(dev_err)?,
            VirtioBlockType::Mmio(blk) => {
                blk.lock().write_blocks(id, buf).await.map_err(dev_err)?
            }
        }
        Ok(buf.len())
    }
}

impl BlockDevice for VirtioBlockType {}
