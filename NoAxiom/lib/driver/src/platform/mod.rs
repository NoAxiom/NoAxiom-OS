pub mod platforminfo;
pub mod plic;

use ksync::Once;
use platforminfo::{platform_info_from_dtb, PlatformInfo};

pub static DTB: Once<usize> = Once::new();

pub fn platform_init(dtb: usize) -> PlatformInfo {
    DTB.call_once(|| dtb);
    let res = platform_info_from_dtb(dtb);
    log::info!("Platform Info:\n {:?}", res);
    res
}

pub fn platform_dtb() -> usize {
    DTB.get().unwrap().clone()
}
