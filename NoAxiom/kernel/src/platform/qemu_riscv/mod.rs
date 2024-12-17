use super::{
    base_riscv::{
        platforminfo::{platform_info_from_dtb, PlatformInfo},
        BaseRISCV,
    },
    DTB,
};
pub struct QemuRISCV;
impl QemuRISCV {
    pub fn new() -> Option<QemuRISCV> {
        Some(QemuRISCV {})
    }
}

impl BaseRISCV for QemuRISCV {
    fn base_info(&self) -> PlatformInfo {
        platform_info_from_dtb(*DTB.get().unwrap())
    }
    fn init_dtb(&self, dtb: Option<usize>) {
        let dtb_ptr = dtb.expect("No dtb found");
        DTB.call_once(|| dtb_ptr);
    }
}
