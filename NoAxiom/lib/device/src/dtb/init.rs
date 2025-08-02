use alloc::string::String;

use driver::basic::{DeviceType, InterruptDeviceType};
use fdt::{node::FdtNode, Fdt};

use crate::dtb::{
    compatible::{OF_PCI_ECAM_TYPE, OF_PLIC_TYPE, OF_VIRTIO_MMIO_TYPE},
    dtb::get_dtb_initializer,
    dtb_addr::init_dtb_addr,
    info::{
        DeviceConfig, DeviceConfigManager, DeviceConfigType, DtbInitializerType, MmioRegion,
        DEV_CONFIG_MANAGER,
    },
};

fn device_init(
    node: &FdtNode,
    info: &mut DeviceConfigManager,
    dev_type: DeviceType,
    conf_type: DeviceConfigType,
) {
    let reg = node.reg().unwrap();
    reg.for_each(|x| {
        let start = x.starting_address as usize;
        let size = x.size.unwrap_or(0);
        info.devices.push(DeviceConfig {
            dev_type,
            region: MmioRegion::new(start, size),
            conf_type,
        });
    });
}

pub const OF_INITIALIZERS: &[(&str, DeviceType, DeviceConfigType)] = &[
    (
        OF_PCI_ECAM_TYPE,
        DeviceType::Unknown,
        DeviceConfigType::PciEcam,
    ),
    (
        OF_VIRTIO_MMIO_TYPE,
        DeviceType::Unknown,
        DeviceConfigType::VirtioMmio,
    ),
    (
        OF_PLIC_TYPE,
        DeviceType::Interrupt(InterruptDeviceType::PLIC),
        DeviceConfigType::Normal,
    ),
];

pub(crate) fn dtb_init_one(node: &FdtNode, info: &mut DeviceConfigManager) -> bool {
    if let Some(of) = node.compatible() {
        for cur_of in of.all().into_iter() {
            for (other_of, dev_type, conf_type) in OF_INITIALIZERS.iter() {
                if cur_of == *other_of {
                    log::info!(
                        "[platform] init node {} with compatible {}",
                        node.name,
                        other_of,
                    );
                    device_init(node, info, *dev_type, *conf_type);
                    return true;
                }
            }
        }
    }
    false
}

fn fdt_init(fdt: Fdt<'static>) {
    let mut info = DeviceConfigManager::new_bare();
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            log::info!("   {}  {}", node.name, compatible.all().collect::<String>());
        } else {
            log::info!("   {}", node.name);
        }
        if !dtb_init_one(&node, &mut info) {
            log::warn!("[platform] no initializer for node {}", node.name,);
        }
    }
    DEV_CONFIG_MANAGER.call_once(|| info);
}

pub fn dtb_init(dtb: usize) {
    init_dtb_addr(dtb);
    match get_dtb_initializer() {
        DtbInitializerType::Ptr(dtb_ptr) => {
            log::info!("[platform] using pointer initializer: {:#x}", dtb_ptr);
            let fdt = unsafe { Fdt::from_ptr(dtb_ptr as *const u8).unwrap() };
            fdt_init(fdt);
        }
        DtbInitializerType::Ref(dtb_ref) => {
            log::info!("[platform] using reference initializer");
            let fdt = Fdt::new(dtb_ref).unwrap();
            fdt_init(fdt);
        }
        DtbInitializerType::Fdt(fdt) => {
            log::info!("[platform] using Fdt initializer");
            fdt_init(fdt)
        }
        DtbInitializerType::Config(config) => {
            log::info!("[platform] using config initializer");
            DEV_CONFIG_MANAGER.call_once(|| DeviceConfigManager::new(config));
        }
    }
}
