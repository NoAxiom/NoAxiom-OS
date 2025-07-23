use alloc::sync::Arc;

use ksync::Once;

use crate::devices::{block::BlockDevice, display::DisplayDevice, net::NetWorkDevice};

// todo: add multi-devices support for single dev type
lazy_static::lazy_static! {
    pub static ref BLK_DEV: Once<Arc<&'static dyn BlockDevice>> = Once::new();
    pub static ref NET_DEV: Once<Arc<&'static dyn NetWorkDevice>> = Once::new();
    pub static ref DISPLAY_DEV: Once<Arc<&'static dyn DisplayDevice>> = Once::new();
}

pub fn get_blk_dev() -> Arc<&'static dyn BlockDevice> {
    Arc::clone(BLK_DEV.get().unwrap())
}

pub fn get_net_dev() -> Arc<&'static dyn NetWorkDevice> {
    Arc::clone(NET_DEV.get().unwrap())
}

pub fn get_display_dev() -> Arc<&'static dyn DisplayDevice> {
    Arc::clone(DISPLAY_DEV.get().unwrap())
}

pub fn register_blk_dev(dev: Arc<&'static dyn BlockDevice>) {
    BLK_DEV.call_once(|| dev);
}

pub fn register_net_dev(dev: Arc<&'static dyn NetWorkDevice>) {
    NET_DEV.call_once(|| dev);
}

pub fn register_display_dev(dev: Arc<&'static dyn DisplayDevice>) {
    DISPLAY_DEV.call_once(|| dev);
}
