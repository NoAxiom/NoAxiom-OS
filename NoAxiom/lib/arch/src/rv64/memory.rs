use core::arch::asm;

use config::mm::PAGE_WIDTH;
use riscv::{asm::sfence_vma_all, register::satp};

use super::RV64;
use crate::ArchMemory;

fn ppn_to_pa(ppn: usize) -> usize {
    ppn << PAGE_WIDTH
}
pub struct PageTable {
    root_ppn: usize,
}
// impl ArchPageTable for PageTable {}

impl ArchMemory for RV64 {
    // type PageTable = PageTable;
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
