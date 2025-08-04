pub type DevResult<T> = Result<T, include::errno::Errno>;

pub trait Device: Sync {
    fn device_name(&self) -> &'static str {
        "Unknown Device"
    }
    fn device_type(&self) -> &'static DeviceType {
        &DeviceType::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDeviceType {
    Virtio,
    VF2Sdcard,
    PhysLA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetDeviceType {
    LoopBack,
    Virtio,
    PhysRV,
    PhysLA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayDeviceType {
    Virtio,
    PhysRV,
    PhysLA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptDeviceType {
    PLIC,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharDeviceType {
    Serial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerDeviceType {
    Virtio,
    PhysRV,
    PhysLA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseDeviceType {
    Virtio,
    PhysRV,
    PhysLA,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Block(BlockDeviceType),
    Net(NetDeviceType),
    Display(DisplayDeviceType),
    Interrupt(InterruptDeviceType),
    Char(CharDeviceType),
    Power(PowerDeviceType),
    Kernel,
    Pending,
    Unknown,
}
