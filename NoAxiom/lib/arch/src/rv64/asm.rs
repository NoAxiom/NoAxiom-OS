use riscv::asm::sfence_vma_all;

#[inline(always)]
pub fn tlb_flush() {
    sfence_vma_all();
}
