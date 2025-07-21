use alloc::string::String;

use fdt::{node::FdtNode, Fdt};

use crate::{
    archs::ARCH_DTB_INITIALIZERS,
    dtb::basic::{DtbInfo, DTB_INFO},
};

fn mmio_init(node: &FdtNode, info: &mut DtbInfo) -> bool {
    if node.name.starts_with(platform::VIRTIO_MMIO_NAME) {
        let reg = node.reg().unwrap();
        reg.for_each(|x| {
            info.virtio_mmio_regions
                .push((x.starting_address as usize, x.size.unwrap()));
        });
        true
    } else {
        false
    }
}

fn pci_init(node: &FdtNode, info: &mut DtbInfo) -> bool {
    if node.name.starts_with(platform::PCI_NAME) {
        let reg = node.reg().unwrap();
        reg.for_each(|x| {
            info.pci_ecam_base = x.starting_address as usize;
        });
        true
    } else {
        false
    }
}

static DTB_INITIALIZERS: &[fn(&FdtNode, &mut DtbInfo) -> bool] = &[mmio_init, pci_init];

pub fn dtb_init(dtb: usize) {
    let fdt = unsafe { Fdt::from_ptr(dtb as *const u8).unwrap() };
    let mut info = DtbInfo::new_bare();
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            log::info!("   {}  {}", node.name, compatible.all().collect::<String>());
        } else {
            log::info!("   {}", node.name);
        }
        for func in DTB_INITIALIZERS {
            if func(&node, &mut info) {
                continue;
            }
        }
        for func in ARCH_DTB_INITIALIZERS {
            if func(&node, &mut info) {
                continue;
            }
        }
    }
    info.virtio_mmio_regions.sort_by(|a, b| a.0.cmp(&b.0));
    DTB_INFO.call_once(|| info);
}
