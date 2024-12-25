#[cfg(feature = "async_fs")]
pub mod async_virtio_driver;
pub mod block;
pub mod console;
pub mod event;
pub mod log;
pub mod probe;
pub mod sbi;
pub mod uart;
mod virtio_drivers2;

use alloc::{fmt::Debug, sync::Arc, vec::Vec};

use block::BlockDriver;
use spin::{Lazy, Mutex};

use crate::{utils::result::Errno, device::IdTable};
/// @brief: Driver error
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DriverError {
    ProbeError,            // 探测设备失败(该驱动不能初始化这个设备)
    RegisterError,         // 设备注册失败
    AllocateResourceError, // 获取设备所需资源失败
    UnsupportedOperation,  // 不支持的操作
    UnInitialized,         // 未初始化
}

impl From<DriverError> for Errno {
    fn from(value: DriverError) -> Self {
        match value {
            DriverError::ProbeError => Errno::ENODEV,
            DriverError::RegisterError => Errno::ENODEV,
            DriverError::AllocateResourceError => Errno::EIO,
            DriverError::UnsupportedOperation => Errno::EIO,
            DriverError::UnInitialized => Errno::ENODEV,
        }
    }
}

pub trait Driver: Sync + Send {
    /// @brief: 获取驱动标识符
    /// @parameter: None
    /// @return: 该驱动驱动唯一标识符
    fn id_table(&self) -> Option<IdTable>;

    fn as_blk(self: Arc<Self>) -> Option<Arc<dyn BlockDriver>>;
}

pub struct DriverManager {
    pub drivers: Vec<Arc<dyn Driver>>,
}
impl DriverManager {
    pub fn push_driver(&mut self, driver: Arc<dyn Driver>) {
        if !self.drivers.iter().any(|d| Arc::ptr_eq(d, &driver)) {
            self.drivers.push(driver);
        }
    }

    pub fn delete_driver(&mut self, driver: &Arc<dyn Driver>) {
        self.drivers.retain(|d| !Arc::ptr_eq(d, driver));
    }
}
pub static DRIVER_MANAGER: Lazy<Mutex<DriverManager>> =
    Lazy::new(|| Mutex::new(DriverManager { drivers: vec![] }));
