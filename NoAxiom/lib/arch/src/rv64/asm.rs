use super::RV64;
use crate::ArchAsm;

impl ArchAsm for RV64 {
    #[inline(always)]
    fn set_hartid(hartid: usize) {
        unsafe { core::arch::asm!("mv tp, {}", in(reg) hartid) }
    }
    #[inline(always)]
    fn get_hartid() -> usize {
        let hartid: usize;
        unsafe { core::arch::asm!("mv {}, tp", out(reg) hartid) }
        hartid
    }
    #[inline(always)]
    fn set_idle() {
        riscv::asm::wfi();
    }
}
