use impls::{BlkDevice, DisplayDevice, NetDevice};
use ksync::cell::SyncUnsafeCell;

pub mod impls;

lazy_static::lazy_static! {
    pub static ref ALL_DEVICES: SyncUnsafeCell<Devices> = SyncUnsafeCell::new(Devices::new());
}

// Provide device drivers implementing various device traits for the kernel
pub struct Devices {
    net: Option<NetDevice>,
    blk: Option<BlkDevice>,
    display: Option<DisplayDevice>,
}

impl Devices {
    pub const DEVICES: usize = 3;
    fn new() -> Self {
        Self {
            net: None,
            blk: None,
            display: None,
        }
    }

    pub fn add_net_device(&mut self, net: NetDevice) {
        self.net = Some(net);
    }
    pub fn add_blk_device(&mut self, blk: BlkDevice) {
        self.blk = Some(blk);
    }
    pub fn add_display_device(&mut self, display: DisplayDevice) {
        self.display = Some(display);
    }
}
