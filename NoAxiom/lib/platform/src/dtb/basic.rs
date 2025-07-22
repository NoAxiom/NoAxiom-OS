use alloc::vec::Vec;

use ksync::Once;

use crate::archs::ArchDtbInfo;
pub struct DtbInfo {
    pub arch: ArchDtbInfo,
    pub virtio_mmio_regions: Vec<(usize, usize)>, // start_addr, size
    pub pci_ecam_base: usize,
}

impl DtbInfo {
    pub fn new_bare() -> Self {
        Self {
            arch: ArchDtbInfo::new(0),
            virtio_mmio_regions: Vec::new(),
            pci_ecam_base: 0,
        }
    }
}

pub static DTB_INFO: Once<DtbInfo> = Once::new();

pub fn dtb_info() -> &'static DtbInfo {
    unsafe { DTB_INFO.get_unchecked() }
}
