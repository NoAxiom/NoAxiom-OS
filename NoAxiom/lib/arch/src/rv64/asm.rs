use super::RV64;
use crate::ArchAsm;

impl ArchAsm for RV64 {
    #[inline(always)]
    fn get_hartid() -> usize {
        let hartid: usize;
        unsafe { core::arch::asm!("mv {}, tp", out(reg) hartid) }
        hartid
    }
    #[inline(always)]
    fn set_idle() {
        unsafe {
            riscv::asm::wfi();
        }
    }
    #[inline(always)]
    fn current_pc() -> usize {
        let pc: usize;
        unsafe { core::arch::asm!("auipc {}, 0", out(reg) pc) }
        pc
    }
}
