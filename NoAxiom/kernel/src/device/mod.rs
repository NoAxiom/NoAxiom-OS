// ignore warnings for this module
#![allow(warnings)]

pub mod block;
pub mod char;
mod config;
pub mod init;
pub mod net;
pub mod random;

use alloc::{
    boxed::Box,
    string::String,
    sync::{Arc, Weak},
};
use core::{future::Future, pin::Pin};

use char::CharDevice;

use crate::{
    alloc::string::ToString,
    device::{block::BlockDevice, config::DeviceNumber},
    driver::Driver,
    include::result::Errno,
};

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum DeviceError {
    DriverExists,         // 设备已存在
    DeviceExists,         // 驱动已存在
    InitializeFailed,     // 初始化错误
    NotInitialized,       // 未初始化的设备
    NoDeviceForDriver,    // 没有合适的设备匹配驱动
    NoDriverForDevice,    // 没有合适的驱动匹配设备
    RegisterError,        // 注册失败
    UnsupportedOperation, // 不支持的操作
}

impl From<DeviceError> for Errno {
    fn from(value: DeviceError) -> Self {
        match value {
            DeviceError::DriverExists => Errno::EEXIST,
            DeviceError::DeviceExists => Errno::EEXIST,
            DeviceError::InitializeFailed => Errno::EIO,
            DeviceError::NotInitialized => Errno::ENODEV,
            DeviceError::NoDeviceForDriver => Errno::ENODEV,
            DeviceError::NoDriverForDevice => Errno::ENODEV,
            DeviceError::RegisterError => Errno::EIO,
            DeviceError::UnsupportedOperation => Errno::EIO,
        }
    }
}
pub type DevResult<T = ()> = Result<T, Errno>;
type Async<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type ADevResult<'a, T = ()> = Async<'a, DevResult<T>>;

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq)]
pub enum DeviceType {
    Bus,
    Net,
    Gpu,
    Input,
    Block,
    Rtc,
    Serial,
    Intc,
    PlatformDev,
    Char,
    Pci,
}

pub trait Device: Send + Sync {
    fn name(&self) -> &str;
    fn dev_type(&self) -> DeviceType;
    /// Register base address
    fn mmio_base(&self) -> usize;
    fn mmio_size(&self) -> usize;
    fn interrupt_number(&self) -> Option<usize>;
    fn interrupt_handler(&self);
    fn init(&self);
    fn driver(&self) -> Option<Arc<dyn Driver>>;
    fn set_driver(&self, driver: Option<Weak<dyn Driver>>);
    fn is_dead(&self) -> bool;
    fn as_blk(self: Arc<Self>) -> Option<Arc<dyn BlockDevice>>;
    fn as_char(self: Arc<Self>) -> Option<Arc<dyn CharDevice>>;
}
pub struct DeviceData {
    pub common: DeviceCommonData,
    pub private: Option<DevicePrivateData>,
}
impl Default for DeviceData {
    fn default() -> Self {
        Self {
            common: DeviceCommonData::default(),
            private: None,
        }
    }
}

pub struct DeviceCommonData {
    pub driver: Option<Weak<dyn Driver>>,
    pub dead: bool,
}
impl Default for DeviceCommonData {
    fn default() -> Self {
        Self {
            driver: None,
            dead: false,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DevicePrivateData {
    id_table: IdTable,
    state: DeviceState,
}
#[allow(dead_code)]
impl DevicePrivateData {
    pub fn new(id_table: IdTable, state: DeviceState) -> Self {
        Self { id_table, state }
    }

    pub fn id_table(&self) -> &IdTable {
        &self.id_table
    }

    pub fn state(&self) -> DeviceState {
        self.state
    }

    pub fn set_state(&mut self, state: DeviceState) {
        self.state = state;
    }
}
/// @brief: 设备标识符类型
#[derive(Debug, Clone, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub struct IdTable {
    basename: String,
    id: Option<DeviceNumber>,
}

/// @brief: 设备标识符操作方法集
impl IdTable {
    /// @brief: 创建一个新的设备标识符
    /// @parameter name: 设备名
    /// @parameter id: 设备id
    /// @return: 设备标识符
    pub fn new(basename: String, id: Option<DeviceNumber>) -> IdTable {
        return IdTable { basename, id };
    }

    /// @brief: 将设备标识符转换成name
    /// @parameter None
    /// @return: 设备名
    pub fn name(&self) -> String {
        if self.id.is_none() {
            return self.basename.clone();
        } else {
            let id = self.id.unwrap();
            return format!("{}:{}", id.major().data(), id.minor());
        }
    }

    pub fn device_number(&self) -> DeviceNumber {
        return self.id.unwrap_or_default();
    }
}
impl Default for IdTable {
    fn default() -> Self {
        IdTable::new("unknown".to_string(), None)
    }
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DeviceState {
    NotInitialized = 0,
    Initialized = 1,
    UnDefined = 2,
}
impl From<u32> for DeviceState {
    fn from(state: u32) -> Self {
        match state {
            0 => DeviceState::NotInitialized,
            1 => DeviceState::Initialized,
            _ => todo!(),
        }
    }
}
/// @brief: 将设备状态转换为u32类型
impl From<DeviceState> for u32 {
    fn from(state: DeviceState) -> Self {
        match state {
            DeviceState::NotInitialized => 0,
            DeviceState::Initialized => 1,
            DeviceState::UnDefined => 2,
        }
    }
}
