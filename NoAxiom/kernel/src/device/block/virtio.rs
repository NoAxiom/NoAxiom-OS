use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};

use async_trait::async_trait;
use ksync::mutex::SpinLock;
use virtio_drivers::{device::blk::VirtIOBlk, transport::pci::PciTransport};

// use spin::{Mutex, MutexGuard};

type Mutex<T> = SpinLock<T>;
type MutexGuard<'a, T> = ksync::mutex::SpinLockGuard<'a, T>;

use super::BlockDevice;
use crate::{
    device::{Device, DeviceData, DeviceType},
    driver::{block::virtio::virtio_impl::HalImpl, Driver},
    include::result::Errno,
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
    async fn read(&self, id: usize, buf: &mut [u8]) {
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
    async fn write(&self, id: usize, buf: &[u8]) {
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
    async fn sync_all(&self) {
        unreachable!()
    }
}

pub struct PciVirtio(SpinLock<VirtIOBlk<HalImpl, PciTransport>>);

impl PciVirtio {
    pub fn new(blk: VirtIOBlk<HalImpl, PciTransport>) -> Self {
        Self(SpinLock::new(blk))
    }
}

#[async_trait]
impl BlockDevice for PciVirtio {
    async fn read(&self, id: usize, buf: &mut [u8]) {
        self.0.lock().read_blocks(id, buf).unwrap();
    }
    async fn write(&self, id: usize, buf: &[u8]) {
        self.0.lock().write_blocks(id, buf).unwrap();
    }
    async fn sync_all(&self) {
        unreachable!()
    }
}
