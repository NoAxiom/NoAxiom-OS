pub mod vf2_sdcard;
pub mod virtio_block;
use alloc::boxed::Box;

use crate::{basic::Device, interrupt::InterruptDevice, DevResult};

#[async_trait::async_trait]
#[allow(unused_variables)]
pub trait BlockDevice: Send + Sync + Device + InterruptDevice {
    fn sync_read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        unimplemented!("{} not implement read!", self.device_name())
    }
    fn sync_write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        unimplemented!("{} not implement write!", self.device_name())
    }
    async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        self.sync_read(id, buf)
    }
    async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        self.sync_write(id, buf)
    }
    async fn sync_all(&self) -> DevResult<()> {
        unimplemented!("{} not implement sync_all!", self.device_name())
    }
}
