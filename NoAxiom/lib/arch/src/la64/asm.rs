use core::arch::asm;

use super::LA64;
use crate::ArchAsm;

impl ArchAsm for LA64 {
    fn set_hartid(_hartid: usize) {
        unimplemented!()
        // unsafe { asm!("move {}, $tp", in(reg) hartid) }
    }
    fn get_hartid() -> usize {
        let hartid: usize;
        unsafe { asm!("csrrd {}, 0x20", out(reg) hartid); }
        hartid
    }
    fn set_idle() {
        unsafe { loongArch64::asm::idle() };
    }
}
