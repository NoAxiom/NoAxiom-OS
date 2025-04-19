use alloc::{string::String, vec::Vec};

use fdt::Fdt;
use ksync::Once;

pub struct DtbInfo {
    #[cfg(target_arch = "riscv64")]
    pub plic: usize,
    pub virtio_mmio_regions: Vec<(usize, usize)>,
    pub pci_ecam_base: usize,
}

pub static DTB_INFO: Once<DtbInfo> = Once::new();

pub fn init(dtb: usize) {
    let fdt = unsafe { Fdt::from_ptr(dtb as *const u8).unwrap() };
    let mut plic = 0;
    let mut virtio_mmio_regions = Vec::new();
    let mut pci_ecam_base = 0;
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            log::info!("   {}  {}", node.name, compatible.all().collect::<String>());
        } else {
            log::info!("   {}", node.name);
        }
        if node.name.starts_with(platform::PLIC_NAME) {
            let reg = node.reg().unwrap();
            reg.for_each(|x| plic = x.starting_address as usize);
        } else if node.name.starts_with(platform::VIRTIO_MMIO_NAME) {
            let reg = node.reg().unwrap();
            reg.for_each(|x| {
                virtio_mmio_regions.push((
                    x.starting_address as usize,
                    x.starting_address as usize + x.size.unwrap(),
                ));
            });
        } else if node.name.starts_with(platform::PCI_NAME) {
            let reg = node.reg().unwrap();
            reg.for_each(|x| {
                pci_ecam_base = x.starting_address as usize;
            });
        }
    }
    virtio_mmio_regions.sort_by(|a, b| a.0.cmp(&b.0));
    DTB_INFO.call_once(|| DtbInfo {
        #[cfg(target_arch = "riscv64")]
        plic,
        virtio_mmio_regions,
        pci_ecam_base,
    });
}

pub fn dtb_info() -> &'static DtbInfo {
    DTB_INFO.get().unwrap()
}
