use core::arch::global_asm;

use loongArch64::register::{pwch, pwcl, stlbps, tlbidx, tlbrehi, tlbrentry::set_tlbrentry};

global_asm!(include_str!("./tlb.S"));
extern "C" {
    fn __tlb_refill();
}

#[inline]
pub fn set_tlb_refill_entry(tlbrentry: usize) {
    set_tlbrentry(tlbrentry & 0xFFFF_FFFF_FFFF);
}

/// flush the TLB entry by VirtualAddress
/// currently unused
#[inline]
#[allow(unused)]
pub fn flush_vaddr(va: usize) {
    unsafe {
        core::arch::asm!("dbar 0; invtlb 0x05, $r0, {reg}", reg = in(reg) va);
    }
}

#[inline]
pub fn tlb_flush_all() {
    unsafe {
        core::arch::asm!("dbar 0; invtlb 0x3, $zero, $zero");
    }
}

pub fn tlb_init_inner() {
    // pub const _PS_4K: usize = 0x0c;
    // pub const _PS_16K: usize = 0x0e;
    // pub const _PS_2M: usize = 0x15;
    // pub const _PS_1G: usize = 0x1e;
    // tlbidx::set_ps(PS_4K);
    // stlbps::set_ps(PS_4K);
    // tlbrehi::set_ps(PS_4K);

    // const PTE_WIDTH: usize = 8;
    // const DIR_WIDTH: usize = PAGE_WIDTH - 3;
    // pwcl::set_pte_width(PTE_WIDTH); // 64-bits
    // pwcl::set_ptbase(PAGE_WIDTH);
    // pwcl::set_ptwidth(DIR_WIDTH);

    // pwcl::set_dir1_base(PAGE_WIDTH + DIR_WIDTH);
    // pwcl::set_dir1_width(DIR_WIDTH);

    // pwch::set_dir3_base(PAGE_WIDTH + DIR_WIDTH * 2);
    // pwch::set_dir3_width(DIR_WIDTH);

    // Page Size 4KB
    const PS_4K: usize = 0x0c;
    tlbidx::set_ps(PS_4K);
    stlbps::set_ps(PS_4K);
    tlbrehi::set_ps(PS_4K);

    // Set Page table entry width
    pwcl::set_pte_width(8);
    // Set Page table width and offset
    pwcl::set_ptbase(12);
    pwcl::set_ptwidth(9);
    pwcl::set_dir1_base(21);
    pwcl::set_dir1_width(9);
    pwcl::set_dir2_base(30);
    pwcl::set_dir2_width(9);
    pwch::set_dir3_base(39);
    pwch::set_dir3_width(9);

    set_tlb_refill_entry(__tlb_refill as usize);
}
