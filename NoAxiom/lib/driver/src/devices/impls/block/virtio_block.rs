use alloc::boxed::Box;

use ksync::cell::SyncUnsafeCell;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{mmio::MmioTransport, pci::PciTransport, Transport},
};

use super::BlockDevice;
use crate::devices::impls::{
    device::{DevResult, Device},
    virtio::{dev_err, VirtioHalImpl},
};

pub enum VirtioBlockType {
    Pci(VirtioBlock<PciTransport>),
    Mmio(VirtioBlock<MmioTransport>),
}

/// Sync Virtio block Driver for pci/mmio bus, mark `Send` and `Sync` for async
pub struct VirtioBlock<T: Transport> {
    inner: SyncUnsafeCell<VirtIOBlk<VirtioHalImpl, T>>,
}

unsafe impl<T: Transport> Send for VirtioBlock<T> {}
unsafe impl<T: Transport> Sync for VirtioBlock<T> {}

impl<T: Transport> VirtioBlock<T> {
    pub fn try_new(transport: T) -> DevResult<Self> {
        let inner = VirtIOBlk::new(transport).map_err(dev_err)?;
        Ok(Self {
            inner: SyncUnsafeCell::new(inner),
        })
    }
}

#[async_trait::async_trait]
impl<T: Transport> Device for VirtioBlock<T> {
    fn device_name(&self) -> &'static str {
        "virtio_block"
    }

    /// for BlockDevice, The buffer length must be a non-zero multiple of
    /// [`SECTOR_SIZE`]
    async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        self.inner.ref_mut().read_blocks(id, buf).map_err(dev_err)?;
        Ok(buf.len())
    }

    /// for BlockDevice, The buffer length must be a non-zero multiple of
    /// [`SECTOR_SIZE`]
    async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        self.inner
            .ref_mut()
            .write_blocks(id, buf)
            .map_err(dev_err)?;
        Ok(buf.len())
    }
}

impl<T: Transport> BlockDevice for VirtioBlock<T> {}
