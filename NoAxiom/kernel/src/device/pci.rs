use core::{error, ptr::NonNull};

use arch::{Arch, Platform};
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
use crate::{config::mm::KERNEL_ADDR_OFFSET, driver::block::virtio::virtio_impl::HalImpl};

pub fn init() -> Result<PciTransport, ()> {
    // probe_bus_devices();

    let fdt = Arch::get_dtb(0xffffffc087000000);
    let fdt = unsafe { Fdt::from_ptr(fdt as *mut u8) }.unwrap();
    let mut all_nodes = fdt.all_nodes();

    if let Some(pci_node) = all_nodes.find(|x| x.name.starts_with("pci")) {
        let pci_addr = pci_node.reg().map(|mut x| x.next().unwrap()).unwrap();
        log::info!("PCI Address: {:#p}", pci_addr.starting_address);
        enumerate_pci((pci_addr.starting_address as usize | KERNEL_ADDR_OFFSET) as *mut u8)
    } else {
        Err(())
    }
}

/// Enumerate the PCI devices
fn enumerate_pci(mmconfig_base: *mut u8) -> Result<PciTransport, ()> {
    info!("mmconfig_base = {:#x}", mmconfig_base as usize);

    let mut pci_root = unsafe { PciRoot::new(mmconfig_base, Cam::Ecam) };
    for (device_function, info) in pci_root.enumerate_bus(0) {
        // Skip if it is not a PCI Type0 device (Standard PCI device).
        if info.header_type != HeaderType::Standard {
            continue;
        }
        let (status, command) = pci_root.get_status_command(device_function);
        info!(
            "Found {} at {}, status {:?} command {:?}",
            info, device_function, status, command
        );

        if info.vendor_id == 0x8086 && info.device_id == 0x100e {
            // Detected E1000 Net Card
            info!("  E1000 Net Card");
            pci_root.set_command(
                device_function,
                Command::IO_SPACE | Command::MEMORY_SPACE | Command::BUS_MASTER,
            );
        }
        for i in 0..6 {
            dump_bar_contents(&mut pci_root, device_function, i);
        }

        if let Some(virtio_type) = virtio_device_type(&info) {
            info!("  VirtIO {:?}", virtio_type);

            // Enable the device to use its BARs.
            // pci_root.set_command(
            //     device_function,
            //     Command::IO_SPACE | Command::MEMORY_SPACE | Command::BUS_MASTER,
            // );

            if virtio_type == DeviceType::Block {
                let mut allocator = PciRangeAllocator::new(0x1000_0000, 0x2eff_0000);

                match config_pci_device(&mut pci_root, device_function, &mut Some(allocator)) {
                    Ok(_) => {
                        info!("  Configured PCI device");

                        if let Some(dev) =
                            probe_pci(&mut pci_root, device_function, virtio_type, &info)
                        {
                            info!(
                                "registered a new {:?} device at {}: {:?}",
                                DeviceType::Block,
                                device_function,
                                "blockdev",
                            );
                        }

                        match unsafe {
                            PciTransport::new::<HalImpl>(&mut pci_root, device_function)
                        } {
                            Ok(mut transport) => {
                                info!(
                                    "Detected virtio PCI device with device type {:?}, features:{:?}",
                                    transport.device_type(),
                                    transport.read_device_features(),
                                );
                                match transport.device_type() {
                                    DeviceType::Block => {
                                        return Ok(transport);
                                    }
                                    ty => {
                                        println!("Don't support virtio device type: {:?}", ty);
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("Error creating VirtIO PCI transport: {}", e);
                                panic!("Error creating VirtIO PCI transport: {}", e);
                            }
                        }
                    }
                    Err(_) => {
                        panic!("Error configuring PCI device");
                    }
                }
            }
        }
    }
    Err(())
}

/// Dump bar Contents.
fn dump_bar_contents(root: &mut PciRoot, device_function: DeviceFunction, bar_index: u8) {
    let bar_info = root.bar_info(device_function, bar_index).unwrap();
    if let Some((addr, size)) = bar_info.memory_address_size() {
        if size == 0 {
            return;
        }
        info!(
            "BAR {}: Address = {:#x}, Size = {:#x}",
            bar_index, addr, size
        );
    }
}

pub(crate) fn probe_bus_devices() {
    let base_addr = Arch::get_dtb(0) | KERNEL_ADDR_OFFSET;
    let mut root = unsafe { PciRoot::new(base_addr as *mut u8, Cam::Ecam) };

    // PCI 32-bit MMIO space
    let mut allocator = Some(PciRangeAllocator::new(0x1000_0000, 0x2eff_0000));

    for bus in 0..=0xff as u8 {
        for (bdf, dev_info) in root.enumerate_bus(bus) {
            trace!("PCI {}: {}", bdf, dev_info);
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
                    }
                }
                Err(e) => warn!(
                    "failed to enable PCI device at {}({}): {:?}",
                    bdf, dev_info, e
                ),
            }
        }
    }
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
        .map_err(|e| debug!("failed to create blk driver because {e}"))?;
    debug!("got virtio blk device driver SUCCEED!");

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
