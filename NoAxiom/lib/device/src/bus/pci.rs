use arch::ArchMemory;
use driver::{basic::DevResult, block::virtio_block::VirtioBlockDevice, hal::VirtioHalImpl};
use include::errno::Errno;
use virtio_drivers::transport::{
    pci::{
        bus::{
            BarInfo, Cam, Command, DeviceFunction, DeviceFunctionInfo, HeaderType, MemoryBarType,
            MmioCam, PciRoot,
        },
        virtio_device_type, PciTransport,
    },
    DeviceType, Transport,
};

use super::pci_driver::PciRangeAllocator;
use crate::{
    device::manager::DEV_BUS,
    dtb::devconf::{PCI_BUS_END, PCI_RANGE},
};

const PCI_BAR_NUM: u8 = 6;

fn try_config_pci_device(
    root: &mut PciRoot<MmioCam>,
    bdf: DeviceFunction,
    allocator: &mut Option<PciRangeAllocator>,
) -> DevResult<()> {
    let mut bar = 0;
    while bar < PCI_BAR_NUM {
        let info = root.bar_info(bdf, bar).unwrap();
        if let BarInfo::Memory {
            address_type,
            address,
            size,
            ..
        } = info
        {
            // if the BAR address is not assigned, call the allocator and assign it.
            if size > 0 && address == 0 {
                let new_addr = allocator
                    .as_mut()
                    .expect("No memory ranges available for PCI BARs!")
                    .alloc(size as _)
                    .ok_or(Errno::EINVAL)?;
                if address_type == MemoryBarType::Width32 {
                    root.set_bar_32(bdf, bar, new_addr as _);
                } else if address_type == MemoryBarType::Width64 {
                    root.set_bar_64(bdf, bar, new_addr);
                }
            }
        }

        // read the BAR info again after assignment.
        let info = root.bar_info(bdf, bar).unwrap();
        match info {
            BarInfo::IO { address, size } => {
                if address > 0 && size > 0 {
                    log::debug!("  BAR {}: IO  [{:#x}, {:#x})", bar, address, address + size);
                }
            }
            BarInfo::Memory {
                address_type,
                prefetchable,
                address,
                size,
            } => {
                if address > 0 && size > 0 {
                    log::debug!(
                        "  BAR {}: MEM [{:#x}, {:#x}){}{}",
                        bar,
                        address,
                        address + size as u64,
                        if address_type == MemoryBarType::Width64 {
                            " 64bit"
                        } else {
                            ""
                        },
                        if prefetchable { " pref" } else { "" },
                    );
                }
            }
        }

        bar += 1;
        if info.takes_two_entries() {
            bar += 1;
        }
    }

    // Enable the device.
    let (_status, cmd) = root.get_status_command(bdf);
    root.set_command(
        bdf,
        cmd | Command::IO_SPACE | Command::MEMORY_SPACE | Command::BUS_MASTER,
    );
    Ok(())
}

pub(crate) fn probe_virtio_pci_device(
    root: &mut PciRoot<MmioCam>,
    bdf: DeviceFunction,
    dev_info: &DeviceFunctionInfo,
) -> Option<PciTransport> {
    let dev_type = virtio_device_type(dev_info)?;
    match (dev_type, dev_info.device_id) {
        (DeviceType::Network, 0x1000) | (DeviceType::Network, 0x1040) => {}
        (DeviceType::Block, 0x1001) | (DeviceType::Block, 0x1041) => {}
        (DeviceType::GPU, 0x1050) => {}
        _ => return None,
    }
    log::info!("[pci] found a virtio PCI device at {}: {:?}", bdf, dev_info);
    PciTransport::new::<VirtioHalImpl, MmioCam>(root, bdf).ok()
}

fn register_virtio_pci_device(transport: PciTransport, bdf: DeviceFunction) {
    let dev_type = transport.device_type();
    log::info!(
        "[pci] detect a new virtio device {:?} at {}",
        transport,
        bdf
    );
    match dev_type {
        DeviceType::Block => {
            let blk_dev = VirtioBlockDevice::new(transport);
            DEV_BUS.add_block_device(blk_dev);
        }
        _ => {
            log::warn!("[pci] IGNORED {:?} virtio device at {}", dev_type, bdf);
        }
    }
}

pub(crate) fn probe_pci_bus(pci_ecam_base: usize) {
    let base_vaddr = pci_ecam_base | arch::Arch::IO_ADDR_OFFSET;
    let mut root = unsafe { PciRoot::new(MmioCam::new(base_vaddr as *mut u8, Cam::Ecam)) };

    // PCI 32-bit MMIO space
    let mut allocator = Some(PciRangeAllocator::new(
        PCI_RANGE.0 as u64,
        PCI_RANGE.1 as u64,
    ));

    for bus in 0..=PCI_BUS_END as u8 {
        for (bdf, dev_info) in root.enumerate_bus(bus) {
            log::debug!("PCI {}: {}", bdf, dev_info);
            if dev_info.header_type != HeaderType::Standard {
                continue;
            }
            match try_config_pci_device(&mut root, bdf, &mut allocator) {
                Ok(_) => {
                    // detect a virtio device
                    if let Some(transport) = probe_virtio_pci_device(&mut root, bdf, &dev_info) {
                        register_virtio_pci_device(transport, bdf);
                    } else {
                        log::warn!(
                            "[pci] device at {}({}) is not a valid virtio device, ignored",
                            bdf,
                            dev_info
                        );
                    }
                }
                Err(e) => {
                    log::warn!(
                        "[pci] failed to enable PCI device at {}({}): {:?}",
                        bdf,
                        dev_info,
                        e
                    );
                }
            }
        }
    }
}
