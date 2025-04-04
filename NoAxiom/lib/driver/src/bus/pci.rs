use arch::ArchMemory;
use config::arch::{PCI_BUS_END, PCI_RANGE};
use include::errno::Errno;
use virtio_drivers::transport::{
    pci::{
        bus::{
            BarInfo, Cam, Command, DeviceFunction, DeviceFunctionInfo, HeaderType, MemoryBarType,
            PciRoot,
        },
        PciTransport,
    },
    DeviceType,
};

use super::pci_driver::PciRangeAllocator;
use crate::{
    devices::{
        impls::{
            block::virtio_block::{VirtioBlock, VirtioBlockType},
            device::DevResult,
            virtio::VirtioHalImpl,
        },
        Devices,
    },
    dtb::dtb_info,
};

impl Devices {
    pub(crate) fn probe_pcibus_devices(&mut self) -> DevResult<()> {
        let base_vaddr = dtb_info().pci_ecam_base | arch::Arch::KERNEL_ADDR_OFFSET;
        let mut root = unsafe { PciRoot::new(base_vaddr as *mut u8, Cam::Ecam) };

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
                match config_pci_device(&mut root, bdf, &mut allocator) {
                    Ok(_) => {
                        #[cfg(feature = "async_fs")]
                        {
                            // todo: now async_fs for pci is not supported
                        }
                        if let Some(transport) =
                            probe_pci(&mut root, bdf, DeviceType::Block, &dev_info)
                        {
                            log::info!(
                                "registered a new {:?} device at {}",
                                DeviceType::Block,
                                bdf,
                            );
                            let blk_dev = VirtioBlock::try_new(transport)?;
                            let blk_dev = VirtioBlockType::Pci(blk_dev);
                            self.add_blk_device(blk_dev);
                        }
                    }
                    Err(e) => log::warn!(
                        "failed to enable PCI device at {}({}): {:?}",
                        bdf,
                        dev_info,
                        e
                    ),
                }
            }
        }
        Ok(())
    }
}

const PCI_BAR_NUM: u8 = 6;

fn config_pci_device(
    root: &mut PciRoot,
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

fn probe_pci(
    root: &mut PciRoot,
    bdf: DeviceFunction,
    dev_type: DeviceType,
    dev_info: &DeviceFunctionInfo,
) -> Option<PciTransport> {
    if dev_info.vendor_id != 0x1af4 {
        return None;
    }
    match (dev_type, dev_info.device_id) {
        (DeviceType::Network, 0x1000) | (DeviceType::Network, 0x1040) => {}
        (DeviceType::Block, 0x1001) | (DeviceType::Block, 0x1041) => {}
        (DeviceType::GPU, 0x1050) => {}
        _ => return None,
    }
    PciTransport::new::<VirtioHalImpl>(root, bdf).ok()
}
