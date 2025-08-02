use alloc::{boxed::Box, vec::Vec};

use driver::{
    block::BlockDevice,
    device_cast,
    display::DisplayDevice,
    interrupt::{InterruptControllerDevice, InterruptDevice},
    net::NetWorkDevice,
};
use ksync::mutex::SpinLock;

#[macro_export]
macro_rules! define_global_device {
    ($global_name:ident, $getter:ident, $setter:ident, $trait_name:ident) => {
        static $global_name: ksync::Once<&'static dyn $trait_name> = ksync::Once::new();

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

define_global_device!(
    INT_CTRL_DEV,
    get_int_ctrl_dev,
    set_int_ctrl_dev,
    InterruptControllerDevice
);

pub struct GeneralBus {
    // interrupt devices do not contain intr controller
    pub block: SpinLock<Vec<&'static dyn BlockDevice>>,
    pub display: SpinLock<Vec<&'static dyn DisplayDevice>>,
    pub network: SpinLock<Vec<&'static dyn NetWorkDevice>>,
    pub interrupt: SpinLock<Vec<&'static dyn InterruptDevice>>,
}

impl GeneralBus {
    pub const fn new() -> Self {
        GeneralBus {
            display: SpinLock::new(Vec::new()),
            block: SpinLock::new(Vec::new()),
            network: SpinLock::new(Vec::new()),
            interrupt: SpinLock::new(Vec::new()),
        }
    }
    pub fn add_block_device<T: BlockDevice + 'static>(&self, dev: T) -> &'static dyn BlockDevice {
        let dev: &'static dyn BlockDevice = Box::leak(Box::new(dev));
        let intr = device_cast!(dev, InterruptDevice);
        self.block.lock().push(dev);
        self.interrupt.lock().push(intr);
        dev
    }
    pub fn add_display_device<T: DisplayDevice + 'static>(
        &self,
        dev: T,
    ) -> &'static dyn DisplayDevice {
        let dev: &'static dyn DisplayDevice = Box::leak(Box::new(dev));
        self.display.lock().push(dev);
        dev
    }
    pub fn add_network_device<T: NetWorkDevice + 'static>(
        &self,
        dev: T,
    ) -> &'static dyn NetWorkDevice {
        let dev: &'static dyn NetWorkDevice = Box::leak(Box::new(dev));
        self.network.lock().push(dev);
        dev
    }
    pub fn get_default_block_device(&self) -> Option<&'static dyn BlockDevice> {
        self.block.lock().first().copied()
    }
    pub fn get_default_display_device(&self) -> Option<&'static dyn DisplayDevice> {
        self.display.lock().first().copied()
    }
    pub fn get_default_network_device(&self) -> Option<&'static dyn NetWorkDevice> {
        self.network.lock().first().copied()
    }
}

pub static DEV_BUS: GeneralBus = GeneralBus::new();

pub fn handle_irq() {
    if let Some(dev) = get_int_ctrl_dev() {
        dev.handle_irq().expect("[driver] handle_irq failed");
    }
}
