use alloc::boxed::Box;

use crate::{
    block::BlockDevice, display::DisplayDevice, interrupt::InterruptDevice, net::NetWorkDevice,
};

#[macro_export]
macro_rules! define_global_device {
    ($global_name:ident, $getter:ident, $setter:ident, $trait_name:ident) => {
        lazy_static::lazy_static! {
            static ref $global_name: ksync::Once<&'static dyn $trait_name> = ksync::Once::new();
        }

        pub fn $getter() -> Option<&'static dyn $trait_name> {
            $global_name.get().map(|dev| *dev)
        }

        pub fn $setter<T: $trait_name + 'static>(dev: T) {
            let dev = Box::new(dev);
            let dev: &'static dyn $trait_name = Box::leak(dev);
            $global_name.call_once(|| dev);
        }
    };
}

define_global_device!(DISPLAY_DEV, get_display_dev, set_display_dev, DisplayDevice);
define_global_device!(BLK_DEV, get_blk_dev, set_blk_dev, BlockDevice);
define_global_device!(NET_DEV, get_net_dev, set_net_dev, NetWorkDevice);
define_global_device!(INTR_DEV, get_intr_dev, set_intr_dev, InterruptDevice);
