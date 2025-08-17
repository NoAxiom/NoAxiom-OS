use crate::{
    basic::{DeviceTreeInfo, DeviceType},
    block::{ls_ahci::LsAhciDevice, vf2_sdcard::sdcard::VF2SdcardDevice},
    interrupt::plic::PlicDevice,
    net::ls2k1000_gmac::impls::LsGmacDevice,
    probe::basic::DeviceConfigType,
};

pub(super) const OF_PCI_ECAM_TYPE: &str = "pci-host-ecam-generic";
pub(super) const OF_VIRTIO_MMIO_TYPE: &str = "virtio,mmio";

macro_rules! device {
    ($device:ty) => {
        (
            <$device>::OF_TYPE,
            <$device>::DEVICE_TYPE,
            <$device>::DEVICE_CONFIG_TYPE,
        )
    };
}

pub(super) const OF_INITIALIZERS: &[(&str, &DeviceType, &DeviceConfigType)] = &[
    (
        OF_PCI_ECAM_TYPE,
        &DeviceType::Probe,
        &DeviceConfigType::PciEcam,
    ),
    (
        OF_VIRTIO_MMIO_TYPE,
        &DeviceType::Probe,
        &DeviceConfigType::VirtioMmio,
    ),
    device!(PlicDevice),
    device!(VF2SdcardDevice),
    device!(LsAhciDevice),
    device!(LsGmacDevice),
];
