use driver::{
    basic::{BlockDeviceType, DeviceType, InterruptDeviceType, NetDeviceType},
    block::virtio_block::VirtioBlockDevice,
    interrupt::plic::PlicDevice,
    set_blk_dev, set_intr_dev,
};
use virtio_drivers::transport::{DeviceType as VirtioDevType, Transport};

use crate::{
    bus::pci::probe_pci_bus,
    device::{
        basic::{DeviceConfig, DeviceConfigType, DEV_CONFIG_MANAGER},
        extra::register_extra_devices,
    },
};

fn virtio_mmio_realize(config: &DeviceConfig) {
    let transport = config.region.into_virtio_transport();
    match transport {
        Ok(transport) => match transport.device_type() {
            VirtioDevType::Block => {
                log::info!(
                        "[platform] realize virtio mmio block device: type {:?} @ addr: {:#x}, size: {:#x}",
                        transport.device_type(),
                        config.region.addr,
                        config.region.size
                    );
                let blk = VirtioBlockDevice::new(transport);
                set_blk_dev(blk);
            }
            VirtioDevType::Network => {
                log::info!(
                        "[platform] realize virtio mmio net device: type {:?} @ addr: {:#x}, size: {:#x}",
                        transport.device_type(),
                        config.region.addr,
                        config.region.size
                    );
                log::warn!(
                        "[platform] virtio mmio net device is not implemented yet, skipping realization."
                    );
            }
            _ => {
                log::warn!(
                        "[platform] realize virtio mmio device: unknown type {:?} @ addr: {:#x}, size: {:#x}",
                        transport.device_type(),
                        config.region.addr,
                        config.region.size
                    );
            }
        },
        Err(err) => {
            log::warn!(
                "[platform] failed to realize virtio mmio device: type {:?} @ addr: {:#x}, size: {:#x}, error: {}",
                config.dev_type,
                config.region.addr,
                config.region.size,
                err
            );
        }
    }
}

pub fn dtb_realize() {
    let manager = DEV_CONFIG_MANAGER.get().unwrap();
    for config in manager.devices.iter() {
        match config.conf_type {
            DeviceConfigType::VirtioMmio => {
                virtio_mmio_realize(config);
            }
            DeviceConfigType::PciEcam => {
                log::info!(
                    "[platform] realize PCI ECAM device: type {:?} @ addr: {:#x}, size: {:#x}",
                    config.dev_type,
                    config.region.addr,
                    config.region.size
                );
                let pci_ecam_base = config.region.addr;
                probe_pci_bus(pci_ecam_base);
            }
            DeviceConfigType::Normal => {
                log::info!(
                    "[platform] realize normal device: type {:?} @ addr: {:#x}, size: {:#x}",
                    config.dev_type,
                    config.region.addr,
                    config.region.size
                );
                match config.dev_type {
                    DeviceType::Block(blk_type) => match blk_type {
                        BlockDeviceType::Virtio => {
                            log::warn!("[platform] UNEXPECTED realize virtio block device!!! SKIP device realization");
                            // virtio_mmio_realize(config);
                        }
                        _ => {
                            log::warn!("[platform] UNKNOWN block device!!!");
                        }
                    },
                    DeviceType::Net(net_type) => match net_type {
                        NetDeviceType::Virtio => {
                            virtio_mmio_realize(config);
                        }
                        _ => {
                            log::warn!("[platform] UNKNOWN network device!!!");
                        }
                    },
                    DeviceType::Interrupt(interrupt_type) => match interrupt_type {
                        InterruptDeviceType::PLIC => {
                            set_intr_dev(PlicDevice::new(config.region.addr));
                        }
                    },
                    _ => {
                        log::warn!(
                            "[platform] realize normal device: unknown type {:?} @ addr: {:#x}, size: {:#x}",
                            config.dev_type,
                            config.region.addr,
                            config.region.size
                        );
                    }
                }
            }
        }
    }
}

pub fn device_realize() {
    register_extra_devices();
    dtb_realize();
}
