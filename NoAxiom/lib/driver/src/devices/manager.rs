use ksync::Once;

use crate::devices::{block::BlockDevice, display::DisplayDevice, net::NetWorkDevice};

// todo: add multi-devices support for single dev type
lazy_static::lazy_static! {
    pub static ref BLK_DEV: Once<&'static dyn BlockDevice> = Once::new();
    pub static ref NET_DEV: Once<&'static dyn NetWorkDevice> = Once::new();
    pub static ref DISPLAY_DEV: Once<&'static dyn DisplayDevice> = Once::new();
}

pub fn get_blk_dev() -> &'static dyn BlockDevice {
    *BLK_DEV.get().unwrap()
}

pub fn get_net_dev() -> &'static dyn NetWorkDevice {
    *NET_DEV.get().unwrap()
}

pub fn get_display_dev() -> &'static dyn DisplayDevice {
    *DISPLAY_DEV.get().unwrap()
}

pub fn register_blk_dev(dev: &'static dyn BlockDevice) {
    BLK_DEV.call_once(|| dev);
}

pub fn register_net_dev(dev: &'static dyn NetWorkDevice) {
    NET_DEV.call_once(|| dev);
}

pub fn register_display_dev(dev: &'static dyn DisplayDevice) {
    DISPLAY_DEV.call_once(|| dev);
}
