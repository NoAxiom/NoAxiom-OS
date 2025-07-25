pub trait Device: Sync {
    fn device_name(&self) -> &'static str {
        "Unknown Device"
    }
    fn device_type(&self) -> &'static DeviceType;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDeviceType {
    Virtio,
    PhysRV,
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
    Virtio,
    PhysRV,
    PhysLA,
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
    Unknown,
}
