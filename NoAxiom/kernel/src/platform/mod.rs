use alloc::sync::Arc;

use base::{platform_info::PlatformInfo, BaseRiscv};
use spin::Once;

pub mod base;
pub mod qemu;

static PLATFORM: Once<Arc<dyn BaseRiscv>> = Once::new();
pub static DTB: Once<usize> = Once::new();
static MACHINE_INFO: Once<PlatformInfo> = Once::new();

#[no_mangle]
pub fn init_platform(_hartid: usize, _dtb: usize) {
    #[cfg(feature = "riscv_qemu")]
    {
        PLATFORM.call_once(|| Arc::new(qemu::QemuRiscv::new().unwrap()));
        PLATFORM.get().unwrap().clone().init_dtb(Some(_dtb));
    }
    let machine_info = PLATFORM.get().unwrap().clone().base_info();
    MACHINE_INFO.call_once(|| machine_info);
}

pub fn platform() -> Arc<dyn BaseRiscv> {
    PLATFORM.get().unwrap().clone()
}

pub fn platform_machine_info() -> PlatformInfo {
    MACHINE_INFO.get().unwrap().clone()
}
pub fn platform_dtb_ptr() -> usize {
    DTB.get().unwrap().clone()
}
