use alloc::vec::Vec;
use core::ptr::NonNull;

use driver::basic::DeviceType;
use fdt::Fdt;
use ksync::Once;
use virtio_drivers::transport::mmio::{MmioError, MmioTransport, VirtIOHeader};

pub struct MmioRegion {
    pub addr: usize,
    pub size: usize,
}

impl MmioRegion {
    pub fn new(addr: usize, size: usize) -> Self {
        Self { addr, size }
    }
    pub fn into_virtio_transport(&self) -> Result<MmioTransport, MmioError> {
        let iova = self.addr | arch::consts::IO_ADDR_OFFSET;
        let header = NonNull::new(iova as *mut VirtIOHeader).unwrap();
        unsafe { MmioTransport::new(header, self.size) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceConfigType {
    VirtioMmio,
    PciEcam,
    Normal,
}

pub struct DeviceConfig {
    pub dev_type: DeviceType,
    pub region: MmioRegion,
    pub conf_type: DeviceConfigType,
}

pub struct DeviceConfigManager {
    pub devices: Vec<DeviceConfig>,
}

impl DeviceConfigManager {
    pub fn new_bare() -> Self {
        Self {
            devices: Vec::new(),
        }
    }
    pub fn new(devices: Vec<DeviceConfig>) -> Self {
        Self { devices }
    }
}

pub static DEV_CONFIG_MANAGER: Once<DeviceConfigManager> = Once::new();

pub enum DtbInitializerType {
    Ptr(usize),
    Ref(&'static [u8]),
    Fdt(Fdt<'static>),
    Config(Vec<DeviceConfig>),
}
