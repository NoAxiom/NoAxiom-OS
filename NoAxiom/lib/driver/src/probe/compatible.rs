use crate::{
    basic::{DeviceType, InterruptDeviceType},
    probe::basic::DeviceConfigType,
};

pub(super) const OF_PCI_ECAM_TYPE: &str = "pci-host-ecam-generic";
pub(super) const OF_VIRTIO_MMIO_TYPE: &str = "virtio,mmio";
pub(super) const OF_PLIC_TYPE: &str = "riscv,plic0";

pub(super) const OF_INITIALIZERS: &[(&str, DeviceType, DeviceConfigType)] = &[
    (
        OF_PCI_ECAM_TYPE,
        DeviceType::Pending,
        DeviceConfigType::PciEcam,
    ),
    (
        OF_VIRTIO_MMIO_TYPE,
        DeviceType::Pending,
        DeviceConfigType::VirtioMmio,
    ),
    (
        OF_PLIC_TYPE,
        DeviceType::Interrupt(InterruptDeviceType::PLIC),
        DeviceConfigType::Normal,
    ),
];
