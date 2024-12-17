pub mod base_riscv;
pub mod plic;
pub mod qemu_riscv;
use alloc::sync::Arc;

use base_riscv::BaseRISCV;
use spin::Once;

use crate::platform::base_riscv::platforminfo::PlatformInfo;

static PLATFORM: Once<Arc<dyn BaseRISCV>> = Once::new();
pub static DTB: Once<usize> = Once::new();
static MACHINE_INFO: Once<PlatformInfo> = Once::new();

#[no_mangle]
pub fn init(_hart_id: usize, dtb: usize) {
    #[cfg(feature = "riscv_qemu")]
    {
        let riscv = qemu_riscv::QemuRISCV::new().unwrap();
        PLATFORM.call_once(|| Arc::new(riscv));
        PLATFORM.get().unwrap().clone().init_dtb(Some(dtb));
    }

    #[cfg(feature = "vf2")]
    {
        let riscv = Starfive2_Riscv::starfive2_riscv::new().unwrap();
        PLATFORM.call_once(|| Arc::new(riscv));
        PLATFORM.get().unwrap().clone().init_dtb(None);
    }

    let machine_info = PLATFORM.get().unwrap().clone().base_info();
    MACHINE_INFO.call_once(|| machine_info);
}

// pub fn platform() -> Arc<dyn BaseRISCV> {
//     PLATFORM.get().unwrap().clone()
// }

// pub fn platform_machine_info() -> PlatformInfo {
//     MACHINE_INFO.get().unwrap().clone()
// }
pub fn platform_dtb_ptr() -> usize {
    DTB.get().unwrap().clone()
}
