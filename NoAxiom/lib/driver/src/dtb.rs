use alloc::{string::String, vec::Vec};

use arch::{Arch, DtbInfo as Dtb};
use fdt::Fdt;
use ksync::Once;

pub struct DtbInfo {
    dtb: usize,
    pub plic: usize,
    pub mmio_regions: Vec<(usize, usize)>,
    pub pci_ecam_base: usize,
}

pub static DTB_INFO: Once<DtbInfo> = Once::new();

pub fn init(dtb: usize) {
    let fdt = unsafe { Fdt::from_ptr(dtb as *const u8).unwrap() };
    let mut plic = 0;
    let mut mmio_regions = Vec::new();
    let mut pci_ecam_base = 0;
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            log::info!("   {}  {}", node.name, compatible.all().collect::<String>());
        } else {
            log::info!("   {}", node.name);
        }
        if node.name.starts_with(Arch::plic_name()) {
            let reg = node.reg().unwrap();
            reg.for_each(|x| plic = x.starting_address as usize);
        } else if node.name.starts_with(Arch::virtio_mmio_name()) {
            let reg = node.reg().unwrap();
            reg.for_each(|x| {
                mmio_regions.push((
                    x.starting_address as usize,
                    x.starting_address as usize + x.size.unwrap(),
                ));
            });
        } else if node.name.starts_with(Arch::pci_name()) {
            let reg = node.reg().unwrap();
            reg.for_each(|x| {
                pci_ecam_base = x.starting_address as usize;
            });
        }
    }
    DTB_INFO.call_once(|| DtbInfo {
        dtb,
        plic,
        mmio_regions,
        pci_ecam_base,
    });
}

pub fn dtb_info() -> &'static DtbInfo {
    DTB_INFO.get().unwrap()
}
