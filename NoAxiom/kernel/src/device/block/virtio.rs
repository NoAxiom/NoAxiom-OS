use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};

use spin::{Mutex, MutexGuard};

use super::BlockDevice;
use crate::{
    nix::result::Errno,
    device::{Device, DeviceData, DeviceType},
    driver::Driver,
};

pub struct virtio {
    pub inner: Mutex<DeviceData>,
    pub base_address: usize,
    pub size: usize,
    // pub pos: u64,
}

impl virtio {
    pub fn new(driver: Option<Weak<dyn Driver>>, base_addr: usize, size: usize) -> Self {
        let mut device_data = DeviceData::default();
        device_data.common.driver = driver;
        Self {
            inner: Mutex::new(device_data),
            base_address: base_addr,
            size,
        }
    }
    pub fn inner(&self) -> MutexGuard<DeviceData> {
        self.inner.lock()
    }
}
impl BlockDevice for virtio {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> Result<(), Errno> {
        self.inner()
            .common
            .driver
            .as_mut()
            .unwrap()
            .upgrade()
            .unwrap()
            .as_blk()
            .unwrap()
            .read_block(block_id, buf)
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) -> Result<(), Errno> {
        self.inner()
            .common
            .driver
            .as_mut()
            .unwrap()
            .upgrade()
            .unwrap()
            .as_blk()
            .unwrap()
            .write_block(block_id, buf)
    }

    fn read_async_block(&self, block_id: usize, buf: &mut [u8]) -> Result<(), Errno> {
        self.inner()
            .common
            .driver
            .as_mut()
            .unwrap()
            .upgrade()
            .unwrap()
            .as_blk()
            .unwrap()
            .read_async_block(block_id, buf)
    }

    fn write_async_block(&self, block_id: usize, buf: &[u8]) -> Result<(), Errno> {
        self.inner()
            .common
            .driver
            .as_mut()
            .unwrap()
            .upgrade()
            .unwrap()
            .as_blk()
            .unwrap()
            .write_async_block(block_id, buf)
    }

    fn size(&self) -> usize {
        self.size
    }

    fn flush(&self) -> Result<(), Errno> {
        todo!()
    }
}

impl Device for virtio {
    fn name(&self) -> &str {
        "vfs2d"
    }
    fn dev_type(&self) -> DeviceType {
        DeviceType::Block
    }
    /// Register base address
    fn mmio_base(&self) -> usize {
        self.base_address
    }
    fn mmio_size(&self) -> usize {
        self.size
    }
    fn interrupt_number(&self) -> Option<usize> {
        None
    }
    fn interrupt_handler(&self) {
        panic!();
    }

    fn as_blk(self: Arc<Self>) -> Option<Arc<dyn BlockDevice>> {
        Some(self)
    }

    fn init(&self) {
        // Not init needed
    }

    fn driver(&self) -> Option<Arc<dyn crate::driver::Driver>> {
        let r = self.inner().common.driver.clone()?.upgrade();
        if r.is_none() {
            self.inner().common.driver = None;
        }

        return r;
    }

    fn set_driver(&self, driver: Option<Weak<dyn Driver>>) {
        self.inner().common.driver = driver;
    }

    fn is_dead(&self) -> bool {
        self.inner().common.dead
    }

    fn as_char(self: Arc<Self>) -> Option<Arc<dyn crate::device::char::CharDevice>> {
        None
    }
}

impl crate::fs::blockdevice::BlockDevice for virtio {
    fn read<'a>(&'a self, id: usize, buf: &'a mut [u8]) -> crate::fs::blockdevice::BlockReturn {
        Box::pin(async move {
            self.read_block(id, buf).unwrap();
            Ok(buf.len() as isize)
        })
    }
    fn write<'a>(&'a self, id: usize, buf: &'a [u8]) -> crate::fs::blockdevice::BlockReturn {
        Box::pin(async move {
            self.write_block(id, buf).unwrap();
            Ok(buf.len() as isize)
        })
    }
    fn close(&self) -> Result<(), ()> {
        todo!()
    }
    fn flush(&self) -> Result<(), ()> {
        todo!()
    }
}
