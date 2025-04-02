use core::arch::asm;

use config::mm::PAGE_WIDTH;
use log::info;
use loongArch64::register::{
    badv, era, pwch, pwcl, stlbps, tlbidx, tlbrehi, tlbrentry::set_tlbrentry,
};

// #[naked]
// pub unsafe extern "C" fn tlb_refill() {
//     asm!(
//         "
//         .equ LA_CSR_PGDL,          0x19    /* Page table base address when
// VA[47] = 0 */         .equ LA_CSR_PGDH,          0x1a    /* Page table base
// address when VA[47] = 1 */         .equ LA_CSR_PGD,           0x1b    /* Page
// table base */         .equ LA_CSR_TLBRENTRY,     0x88    /* TLB refill
// exception entry */         .equ LA_CSR_TLBRBADV,      0x89    /* TLB refill
// badvaddr */         .equ LA_CSR_TLBRERA,       0x8a    /* TLB refill ERA */
//         .equ LA_CSR_TLBRSAVE,      0x8b    /* KScratch for TLB refill
// exception */         .equ LA_CSR_TLBRELO0,      0x8c    /* TLB refill
// entrylo0 */         .equ LA_CSR_TLBRELO1,      0x8d    /* TLB refill entrylo1
// */         .equ LA_CSR_TLBREHI,       0x8e    /* TLB refill entryhi */
//         .balign 4096
//             csrwr   $t0, LA_CSR_TLBRSAVE
//             csrrd   $t0, LA_CSR_PGD
//             lddir   $t0, $t0, 3
//             lddir   $t0, $t0, 1
//             ldpte   $t0, 0
//             ldpte   $t0, 1
//             tlbfill
//             csrrd   $t0, LA_CSR_TLBRSAVE
//             ertn
//         ",
//         options(noreturn)
//     );
// }

#[naked]
pub unsafe extern "C" fn tlb_refill_new() {
    asm!(
        // PGD: 0x1b CRMD:0x0 PWCL:0x1c TLBRBADV:0x89 TLBERA:0x8a TLBRSAVE:0x8b SAVE:0x30
        // TLBREHi: 0x8e STLBPS: 0x1e MERRsave:0x95
        "
            .balign 4096
            csrwr  $t0, 0x8b

            csrrd  $t0, 0x1b
            lddir  $t0, $t0, 3
            andi   $t0, $t0, 1
            beqz   $t0, 1f

            csrrd  $t0, 0x1b
            lddir  $t0, $t0, 3
            addi.d $t0, $t0, -1
            lddir  $t0, $t0, 1
            andi   $t0, $t0, 1
            beqz   $t0, 1f
            csrrd  $t0, 0x1b
            lddir  $t0, $t0, 3
            addi.d $t0, $t0, -1
            lddir  $t0, $t0, 1
            addi.d $t0, $t0, -1

            ldpte  $t0, 0
            ldpte  $t0, 1
            csrrd  $t0, 0x8c
            csrrd  $t0, 0x8d
            csrrd  $t0, 0x0
        2:
            tlbfill
            csrrd  $t0, 0x89
            srli.d $t0, $t0, 13
            slli.d $t0, $t0, 13
            csrwr  $t0, 0x11
            tlbsrch
            tlbrd
            csrrd  $t0, 0x12
            csrrd  $t0, 0x13
            csrrd  $t0, 0x8b
            ertn
        1:
            csrrd  $t0, 0x8e
            ori    $t0, $t0, 0xC
            csrwr  $t0, 0x8e

            rotri.d $t0, $t0, 61
            ori    $t0, $t0, 3
            rotri.d $t0, $t0, 3

            csrwr  $t0, 0x8c
            csrrd  $t0, 0x8c
            csrwr  $t0, 0x8d
            b      2b
        ",
        options(noreturn)
    )
}

#[repr(align(4096))]
pub fn tlb_refill111() {
    info!(
        "refill, era = {}, badv = {}",
        era::read().pc(),
        badv::read().vaddr()
    );
    unsafe {
        tlb_refill_new();
    }
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
        core::arch::asm!("invtlb 0x0, $r0, $r0");
    }
}

pub const PS_4K: usize = 0x0c;
pub const _PS_16K: usize = 0x0e;
pub const _PS_2M: usize = 0x15;
pub const _PS_1G: usize = 0x1e;

pub const PAGE_SIZE_SHIFT: usize = PAGE_WIDTH;

pub fn tlb_init_inner() {
    // // setup PWCTL
    // unsafe {
    // asm!(
    //     "li.d     $r21,  0x4d52c",     // (9 << 15) | (21 << 10) | (9 << 5) | 12
    //     "csrwr    $r21,  0x1c",        // LOONGARCH_CSR_PWCTL0
    //     "li.d     $r21,  0x25e",       // (9 << 6)  | 30
    //     "csrwr    $r21,  0x1d",         //LOONGARCH_CSR_PWCTL1
    //     )
    // }

    tlbidx::set_ps(PS_4K);
    stlbps::set_ps(PS_4K);
    tlbrehi::set_ps(PS_4K);

    // set hardware
    pwcl::set_pte_width(8); // 64-bits
    pwcl::set_ptbase(PAGE_SIZE_SHIFT);
    pwcl::set_ptwidth(PAGE_SIZE_SHIFT - 3);

    pwcl::set_dir1_base(PAGE_SIZE_SHIFT + PAGE_SIZE_SHIFT - 3);
    pwcl::set_dir1_width(PAGE_SIZE_SHIFT - 3);

    pwch::set_dir3_base(PAGE_SIZE_SHIFT + PAGE_SIZE_SHIFT - 3 + PAGE_SIZE_SHIFT - 3);
    pwch::set_dir3_width(PAGE_SIZE_SHIFT - 3);

    set_tlb_refill_entry(tlb_refill111 as usize);
    // pgdl::set_base(kernel_pgd_base);
    // pgdh::set_base(kernel_pgd_base);
    // tlb_flush_all();
}
