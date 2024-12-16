use super::{
    base::{
        platform_info::{platform_info_from_dtb, PlatformInfo},
        BaseRiscv,
    },
    DTB,
};

pub struct QemuRiscv;
impl QemuRiscv {
    pub fn new() -> Option<QemuRiscv> {
        Some(QemuRiscv {})
    }
}

impl BaseRiscv for QemuRiscv {
    fn base_info(&self) -> PlatformInfo {
        platform_info_from_dtb(*DTB.get().unwrap())
    }
    fn init_dtb(&self, dtb: Option<usize>) {
        let dtb_ptr = dtb.expect("No dtb found");

        DTB.call_once(|| dtb_ptr);
    }
}
