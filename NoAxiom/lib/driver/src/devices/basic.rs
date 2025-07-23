use crate::devices::{block::BlockDriverType, display::DisplayDriverType, net::NetDriverType};

pub trait Device {
    fn device_name(&self) -> &'static str {
        "Unknown Device"
    }
    fn driver_type(&self) -> DriverType;
}

pub enum DriverType {
    Block(BlockDriverType),
    NetWork(NetDriverType),
    Display(DisplayDriverType),
    Unknown,
}
