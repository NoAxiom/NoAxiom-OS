//! ACHI device implementation for Loongson 2K1000 series.
//!
//! ahci@400e0000 {
//!     compatible = "loongson,ls-ahci";
//!     reg = <0x00000000 0x400e0000 0x00000000 0x00010000>;
//!     interrupt-parent = <0x00000003>;
//!     interrupts = <0x00000013>;
//!     dma-mask = <0x00000000 0xffffffff>;
//! };

use driver_ahci::libahci::AhciDevice;

use crate::{
    basic::{BlockDeviceType, DevResult, Device, DeviceTreeInfo, DeviceType},
    block::BlockDevice,
    interrupt::InterruptDevice,
    probe::basic::DeviceConfigType,
};

pub struct LsAhciDevice {
    device: AhciDevice,
}

const DEVICE_TYPE: DeviceType = DeviceType::Block(BlockDeviceType::LS2k1000Ahci);

impl Device for LsAhciDevice {
    fn device_name(&self) -> &'static str {
        "ls2k1000-ahci-blk"
    }
    fn device_type(&self) -> &'static DeviceType {
        &DEVICE_TYPE
    }
}

impl DeviceTreeInfo for LsAhciDevice {
    const DEVICE_CONFIG_TYPE: &'static DeviceConfigType = &DeviceConfigType::DeviceTree;
    const DEVICE_TYPE: &'static DeviceType = &DEVICE_TYPE;
    const OF_TYPE: &'static str = "loongson,ls-ahci";
}

impl LsAhciDevice {
    pub fn new(mut device: AhciDevice) -> Self {
        device.ahci_init();
        LsAhciDevice { device }
    }
}

impl InterruptDevice for LsAhciDevice {
    fn handle_irq(&self) -> DevResult<()> {
        unimplemented!()
    }
}

impl BlockDevice for LsAhciDevice {
    fn sync_read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        unimplemented!()
        // self.device.ahci_sata_read_common(blknr, blkcnt, buffer)
    }

    fn sync_write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        unimplemented!()
        // self.device.ahci_sata_write_common(blknr, blkcnt, buffer)
    }
}
