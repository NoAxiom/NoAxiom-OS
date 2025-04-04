// //!  VirtIO 总线架构下的块设备

use alloc::vec::Vec;
use core::ptr::NonNull;

use ksync::mutex::SpinLock;
use platform::qemu::VIRTIO0;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::mmio::{MmioTransport, VirtIOHeader},
};

use super::virtio_impl::HalImpl;
use crate::{
    config::fs::BLOCK_SIZE,
    driver::{
        block::BlockDriver,
        probe::{ProbeInfo, PROBE},
        Driver,
    },
    include::result::Errno,
};

pub struct VirtIOBlockDriver {
    driver: SpinLock<VirtIOBlk<HalImpl, MmioTransport>>,
}

pub fn probe() -> Option<Vec<ProbeInfo>> {
    if cfg!(any(feature = "qemu_riscv")) {
        return None;
    }
    PROBE.get().unwrap().probe_virtio()
}
impl BlockDriver for VirtIOBlockDriver {
    fn read_block(&self, blk_id: usize, buf: &mut [u8]) -> Result<(), Errno> {
        assert_eq!(buf.len(), BLOCK_SIZE);
        self.driver.lock().read_blocks(blk_id, buf).unwrap();
        Ok(())
    }

    fn write_block(&self, blk_id: usize, buf: &[u8]) -> Result<(), Errno> {
        self.driver.lock().write_blocks(blk_id, buf).unwrap();
        Ok(())
    }

    fn read_async_block(&self, _block_id: usize, _buf: &mut [u8]) -> Result<(), Errno> {
        todo!()
    }

    fn write_async_block(&self, _block_id: usize, _buf: &[u8]) -> Result<(), Errno> {
        todo!()
    }

    fn size(&self) -> usize {
        let res = self.driver.lock().capacity() as usize;
        res
    }

    fn flush(&self) -> Result<(), Errno> {
        todo!()
    }

    fn handle_irq(&self) {
        todo!()
    }
}

impl Driver for VirtIOBlockDriver {
    fn id_table(&self) -> Option<crate::device::IdTable> {
        None
    }

    fn as_blk(self: alloc::sync::Arc<Self>) -> Option<alloc::sync::Arc<dyn BlockDriver>> {
        Some(self)
    }
}
impl VirtIOBlockDriver {
    pub fn new() -> Self {
        let header = NonNull::new(VIRTIO0 as *mut VirtIOHeader).unwrap();
        let blk = match unsafe { MmioTransport::new(header) } {
            Err(e) => {
                panic!("Error creating VirtIO MMIO transport: {}", e)
            }
            Ok(transport) => VirtIOBlk::<HalImpl, MmioTransport>::new(transport)
                .expect("failed to create blk driver"),
        };
        Self {
            driver: SpinLock::new(blk),
        }
    }

    pub fn from_mmio(mmio_transport: MmioTransport) -> Self {
        let blk = VirtIOBlk::<HalImpl, MmioTransport>::new(mmio_transport)
            .expect("failed to create blk driver");
        Self {
            driver: SpinLock::new(blk),
        }
    }
}
