use super::LA64;
use crate::ArchAsm;

impl ArchAsm for LA64 {
    fn set_hartid(hartid: usize) {
        unsafe { core::arch::asm!("move {}, $r21", in(reg) hartid) }
    }
    fn get_hartid() -> usize {
        let hartid: usize;
        unsafe { core::arch::asm!("move $r21, {}", out(reg) hartid) }
        hartid
    }
    fn set_idle() {
        unsafe { loongArch64::asm::idle() };
    }
}
