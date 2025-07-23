use ksync::Once;

use crate::{archs::dtb::ArchDtbInfo, dtb::virtio::DtbVirtioInfo};

pub struct DtbInfo {
    pub arch: ArchDtbInfo,
    pub virtio: DtbVirtioInfo,
}

impl DtbInfo {
    pub fn new_bare() -> Self {
        Self {
            arch: ArchDtbInfo::new(0),
            virtio: DtbVirtioInfo::new(),
        }
    }
    pub fn normalize(&mut self) {
        self.virtio.normalize();
    }
}

pub static DTB_INFO: Once<DtbInfo> = Once::new();

pub fn dtb_info() -> &'static DtbInfo {
    DTB_INFO.get().unwrap()
}
