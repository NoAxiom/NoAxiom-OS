use core::arch::asm;

use riscv::{asm::sfence_vma_all, register::satp};

use super::RV64;
use crate::ArchMemory;

impl ArchMemory for RV64 {
    // flush all TLB
    #[inline(always)]
    fn tlb_flush() {
        sfence_vma_all();
    }
    // update page table base address
    #[inline(always)]
    fn update_pagetable(bits: usize) {
        satp::write(bits);
    }
    #[inline(always)]
    fn get_token_by_ppn(ppn: usize) -> usize {
        8usize << 60 | ppn
    }
    #[inline(always)]
    fn current_token() -> usize {
        let satp: usize;
        unsafe { asm!("csrr {}, satp", out(reg) satp) }
        satp
    }
}
