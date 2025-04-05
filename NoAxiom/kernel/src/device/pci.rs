use core::{error, ptr::NonNull};

use arch::{consts::KERNEL_ADDR_OFFSET, Arch, DtbInfo};
use fdt::Fdt;
use log::{debug, info, trace};
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        pci::{
            bus::{
                BarInfo, Cam, Command, DeviceFunction, DeviceFunctionInfo, HeaderType,
                MemoryBarType, PciRoot,
            },
            virtio_device_type, PciTransport,
        },
        DeviceType, Transport,
    },
    Hal,
};

use super::pci_driver::PciRangeAllocator;
use crate::{driver::block::virtio::virtio_impl::HalImpl, platform::DTB};

pub fn init() -> Result<VirtIOBlk<HalImpl, PciTransport>, ()> {
    let fdt = *DTB.get().unwrap();
    let fdt = unsafe { Fdt::from_ptr(fdt as *mut u8) }.unwrap();
    let mut all_nodes = fdt.all_nodes();

    if let Some(pci_node) = all_nodes.find(|x| x.name.starts_with("pci")) {
        let pci_addr = pci_node.reg().map(|mut x| x.next().unwrap()).unwrap();
        log::info!("PCI Address: {:#p}", pci_addr.starting_address);
        probe_bus_devices((pci_addr.starting_address as usize | KERNEL_ADDR_OFFSET))
    } else {
        Err(())
    }
}

pub(crate) fn probe_bus_devices(mmiobase: usize) -> Result<VirtIOBlk<HalImpl, PciTransport>, ()> {
    let mut root = unsafe { PciRoot::new(mmiobase as *mut u8, Cam::Ecam) };

    // PCI 32-bit MMIO space
    let mut allocator = Some(PciRangeAllocator::new(0x4000_0000, 0x0002_0000));

    for bus in 0..=0 as u8 {
        for (bdf, dev_info) in root.enumerate_bus(bus) {
            info!("PCI {}: {}", bdf, dev_info);
            if dev_info.header_type != HeaderType::Standard {
                continue;
            }
            match config_pci_device(&mut root, bdf, &mut allocator) {
                Ok(_) => {
                    if let Some(dev) = probe_pci(&mut root, bdf, DeviceType::Block, &dev_info) {
                        info!(
                            "registered a new {:?} device at {}: {:?}",
                            DeviceType::Block,
                            bdf,
                            "blockdev",
                        );
                        return Ok(dev);
                    }
                }
                Err(e) => {
                    warn!(
                        "failed to enable PCI device at {}({}): {:?}",
                        bdf, dev_info, e
                    );
                }
            }
        }
    }
    Err(())
}

fn probe_pci(
    root: &mut PciRoot,
    bdf: DeviceFunction,
    dev_type: DeviceType,
    dev_info: &DeviceFunctionInfo,
) -> Option<VirtIOBlk<HalImpl, PciTransport>> {
    if dev_info.vendor_id != 0x1af4 {
        return None;
    }
    match (dev_type, dev_info.device_id) {
        (DeviceType::Network, 0x1000) | (DeviceType::Network, 0x1040) => {}
        (DeviceType::Block, 0x1001) | (DeviceType::Block, 0x1041) => {}
        (DeviceType::GPU, 0x1050) => {}
        _ => return None,
    }

    if let Some((ty, transport)) = probe_pci_device::<HalImpl>(root, bdf, dev_info) {
        if ty == dev_type {
            debug!("Found virtio blk device at {}: {:?}", bdf, ty);
            match try_new(transport) {
                Ok(dev) => return Some(dev),
                Err(e) => {
                    warn!(
                        "failed to initialize PCI device at {}({}): {:?}",
                        bdf, dev_info, e
                    );
                    return None;
                }
            }
        }
    }
    None
}

fn try_new(transport: PciTransport) -> Result<VirtIOBlk<HalImpl, PciTransport>, ()> {
    debug!(
        "getting virtio blk device driver, transport: {:?}!",
        transport
    );
    let res = VirtIOBlk::<HalImpl, PciTransport>::new(transport)
        .map_err(|e| error!("failed to create blk driver because {e}"))?;

    Ok(res)
}

/// Try to probe a VirtIO PCI device from the given PCI address.
///
/// If the device is recognized, returns the device type and a transport object
/// for later operations. Otherwise, returns [`None`].
pub fn probe_pci_device<H: Hal>(
    root: &mut PciRoot,
    bdf: DeviceFunction,
    dev_info: &DeviceFunctionInfo,
) -> Option<(DeviceType, PciTransport)> {
    use virtio_drivers::transport::pci::virtio_device_type;

    let dev_type = virtio_device_type(dev_info).unwrap();
    let transport = PciTransport::new::<H>(root, bdf).ok()?;
    Some((dev_type, transport))
}

const PCI_BAR_NUM: u8 = 6;

fn config_pci_device(
    root: &mut PciRoot,
    bdf: DeviceFunction,
    allocator: &mut Option<PciRangeAllocator>,
) -> Result<(), ()> {
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
                    .ok_or(())?;
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
                    debug!("  BAR {}: IO  [{:#x}, {:#x})", bar, address, address + size);
                } else {
                    debug!("  BAR {}: IO  [Not assigned]", bar);
                }
            }
            BarInfo::Memory {
                address_type,
                prefetchable,
                address,
                size,
            } => {
                if address > 0 && size > 0 {
                    debug!(
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
