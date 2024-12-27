use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};

use async_trait::async_trait;
use spin::{Mutex, MutexGuard};

use super::BlockDevice;
use crate::{
    device::{Device, DeviceData, DeviceType},
    driver::Driver,
    nix::result::Errno,
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

#[async_trait]
impl BlockDevice for virtio {
    async fn read<'a>(&'a self, id: usize, buf: &'a mut [u8]) {
        self.inner()
            .common
            .driver
            .as_mut()
            .unwrap()
            .upgrade()
            .unwrap()
            .as_blk()
            .unwrap()
            .read_block(id, buf);
    }
    async fn write<'a>(&'a self, id: usize, buf: &'a [u8]) {
        self.inner()
            .common
            .driver
            .as_mut()
            .unwrap()
            .upgrade()
            .unwrap()
            .as_blk()
            .unwrap()
            .write_block(id, buf);
    }
}
