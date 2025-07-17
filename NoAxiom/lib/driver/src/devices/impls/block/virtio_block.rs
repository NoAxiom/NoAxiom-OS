use alloc::boxed::Box;

use ksync::cell::SyncUnsafeCell;
use virtio_drivers_async::{
    device::blk::VirtIOBlk,
    transport::{mmio::MmioTransport, pci::PciTransport},
};

use crate::devices::impls::{
    device::{BlockDevice, DevResult},
    virtio::{dev_err, VirtioHalImpl},
};

pub enum VirtioBlockType {
    Pci(SyncUnsafeCell<VirtIOBlk<VirtioHalImpl, PciTransport>>),
    Mmio(SyncUnsafeCell<VirtIOBlk<VirtioHalImpl, MmioTransport>>),
}

#[async_trait::async_trait]
impl BlockDevice for VirtioBlockType {
    fn device_name(&self) -> &'static str {
        match self {
            VirtioBlockType::Pci(_) => "virtio_block_pci",
            VirtioBlockType::Mmio(_) => "virtio_block_mmio",
        }
    }
    async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        match self {
            VirtioBlockType::Pci(blk) => blk
                .as_ref_mut()
                .read_blocks(id, buf)
                .await
                .map_err(dev_err)?,
            VirtioBlockType::Mmio(blk) => blk
                .as_ref_mut()
                .read_blocks(id, buf)
                .await
                .map_err(dev_err)?,
        }
        Ok(buf.len())
    }
    async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        match self {
            VirtioBlockType::Pci(blk) => blk
                .as_ref_mut()
                .write_blocks(id, buf)
                .await
                .map_err(dev_err)?,
            VirtioBlockType::Mmio(blk) => blk
                .as_ref_mut()
                .write_blocks(id, buf)
                .await
                .map_err(dev_err)?,
        }
        Ok(buf.len())
    }
}
