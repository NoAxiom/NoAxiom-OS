use alloc::string::String;

use fdt::{node::FdtNode, Fdt};

use crate::{
    archs::dtb::{get_dtb, ARCH_OF_INITIALIZERS},
    dtb::{
        basic::{DtbInfo, DTB_INFO},
        virtio::DtbVirtioRegion,
    },
};

pub(crate) const OF_VIRTIO_MMIO_TYPE: &str = "virtio,mmio";
pub(crate) fn virtio_mmio_init(node: &FdtNode, info: &mut DtbInfo) {
    let reg = node.reg().unwrap();
    reg.for_each(|x| {
        let start = x.starting_address as usize;
        let size = x.size.unwrap();
        info.virtio
            .mmio_regions
            .push(DtbVirtioRegion::new(start, size));
    });
}

pub(crate) const OF_PCI_ECAM_TYPE: &str = "pci-host-ecam-generic";
pub(crate) fn pci_init(node: &FdtNode, info: &mut DtbInfo) {
    let reg = node.reg().unwrap();
    reg.for_each(|x| {
        info.virtio.pci_ecam.push(x.starting_address as usize);
    });
}

pub const OF_INITIALIZERS: &[(&str, fn(&FdtNode, &mut DtbInfo))] = &[
    (OF_PCI_ECAM_TYPE, pci_init),
    (OF_VIRTIO_MMIO_TYPE, virtio_mmio_init),
];

pub(crate) fn dtb_init_one(
    node: &FdtNode,
    info: &mut DtbInfo,
    table: &[(&str, fn(&FdtNode, &mut DtbInfo))],
) -> bool {
    if let Some(of) = node.compatible() {
        for cur_of in of.all().into_iter() {
            for (other_of, func) in table {
                if cur_of == *other_of {
                    log::info!(
                        "[platform] init node {} with compatible {}",
                        node.name,
                        other_of,
                    );
                    func(node, info);
                    return true;
                }
            }
        }
    }
    false
}

pub fn dtb_init(dtb: usize) {
    let dtb = get_dtb(dtb) | arch::consts::KERNEL_ADDR_OFFSET;
    log::debug!("[platform] init with dtb: {:#x}", dtb);

    let fdt = unsafe { Fdt::from_ptr(dtb as *const u8).unwrap() };
    let mut info = DtbInfo::new_bare();
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            log::info!("   {}  {}", node.name, compatible.all().collect::<String>());
        } else {
            log::info!("   {}", node.name);
        }
        if !dtb_init_one(&node, &mut info, OF_INITIALIZERS) {
            dtb_init_one(&node, &mut info, ARCH_OF_INITIALIZERS);
        }
    }

    info.normalize();
    DTB_INFO.call_once(|| info);
}
