pub mod platforminfo;
pub mod plic;

use ksync::Once;
use platforminfo::platform_info_from_dtb;
pub static DTB: Once<usize> = Once::new();

#[no_mangle]
pub fn platform_init(dtb: usize) -> platforminfo::PlatformInfo {
    DTB.call_once(|| dtb);
    let res = platform_info_from_dtb(dtb);
    debug!("Platform Info:\n {:?}", res);
    res
}

pub fn platform_dtb_ptr() -> usize {
    DTB.get().unwrap().clone()
}
