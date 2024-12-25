use alloc::sync::{Arc, Weak};

use spin::Mutex;

use super::BlockDevice;
use crate::{
    nix::result::Errno,
    device::{Device, DeviceType},
    driver::Driver,
    println,
};

pub struct MemoryFat32Img {
    // data: &'static mut [u8],
    pub inner: Mutex<&'static mut [u8]>,
    // pub base_address: usize,
    pub size: usize,
    // pub pos: u64,
}

impl MemoryFat32Img {
    // pub fn new(data: &'static mut [u8]) -> Self {
    //     Self {
    //         inner: Mutex::new(data),
    //         size: 0x10000,
    //     }
    // }
    // pub fn inner(&self) -> MutexGuard<DeviceData> {
    //     self.inner.lock()
    // }
}
impl BlockDevice for MemoryFat32Img {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> Result<(), Errno> {
        println!("MemoryFat32Img read_block");
        println!("block_id {:?} buf len {:?}", block_id, buf.len());
        let start = block_id * 512;
        let end = start + 512;
        buf.copy_from_slice(&self.inner.lock()[start..end]);
        Ok(())
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) -> Result<(), Errno> {
        let start = block_id * 512;
        let end = start + 512;
        self.inner.lock()[start..end].copy_from_slice(buf);
        Ok(())
    }

    fn read_async_block(&self, block_id: usize, buf: &mut [u8]) -> Result<(), Errno> {
        let start = block_id * 512;
        let end = start + 512;
        buf.copy_from_slice(&self.inner.lock()[start..end]);
        Ok(())
    }

    fn write_async_block(&self, block_id: usize, buf: &[u8]) -> Result<(), Errno> {
        let start = block_id * 512;
        let end = start + 512;
        self.inner.lock()[start..end].copy_from_slice(buf);
        Ok(())
    }

    fn size(&self) -> usize {
        self.size
    }

    fn flush(&self) -> Result<(), Errno> {
        todo!()
    }
}

impl Device for MemoryFat32Img {
    fn name(&self) -> &str {
        "MemoryFat32Img"
    }
    fn dev_type(&self) -> DeviceType {
        DeviceType::Block
    }
    /// Register base address
    fn mmio_base(&self) -> usize {
        // self.base_address
        0x10000
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
        // let r = self.inner().common.driver.clone()?.upgrade();
        // if r.is_none() {
        //     self.inner().common.driver = None;
        // }

        // return r;
        None
    }

    fn set_driver(&self, _driver: Option<Weak<dyn Driver>>) {
        // self.inner().common.driver = _driver;
    }

    fn is_dead(&self) -> bool {
        // self.inner().common.dead
        false
    }

    fn as_char(self: Arc<Self>) -> Option<Arc<dyn crate::device::char::CharDevice>> {
        None
    }
}
