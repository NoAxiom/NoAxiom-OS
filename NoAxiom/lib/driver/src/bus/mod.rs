use alloc::sync::Arc;

use ksync::Lazy;

use crate::{
    bus::{mmio::probe_mmiobus_devices, pci::probe_pcibus_devices},
    devices::{
        block::BlockDevice,
        net::{loopback::LoopBackDev, NetWorkDevice},
    },
    BLK_DEV, NET_DEV,
};

mod mmio;
mod pci;
mod pci_driver;

pub fn probe_bus() {
    probe_mmiobus_devices().map(|dev| {
        log::debug!("[driver] probe mmio bus");
        BLK_DEV.call_once(|| Arc::new(&*dev as &'static dyn BlockDevice));
    });
    probe_pcibus_devices().map(|dev| {
        log::debug!("[driver] probe pci bus");
        BLK_DEV.call_once(|| Arc::new(&*dev as &'static dyn BlockDevice));
    });
    NET_DEV.call_once(|| {
        static LOOPBACK: Lazy<LoopBackDev> = Lazy::new(|| LoopBackDev::new());
        Arc::new(&*LOOPBACK as &'static dyn NetWorkDevice)
    });
    log::debug!("[driver] probe succeed");
}
