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
}
