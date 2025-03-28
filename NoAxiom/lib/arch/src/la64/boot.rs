use core::arch::asm;

use config::{arch::CPU_NUM, mm::KERNEL_STACK_SIZE};
use loongArch64::consts::LOONGARCH_CSR_MAIL_BUF0;

use super::LA64;
use crate::ArchTrap;

extern "C" {
    fn _boot_hart_init();
    fn _other_hart_init();
}

/// temp stack for kernel booting
#[link_section = ".bss.kstack"]
pub(crate) static BOOT_STACK: [u8; KERNEL_STACK_SIZE * CPU_NUM] = [0; KERNEL_STACK_SIZE * CPU_NUM];

// /// temp page table for kernel booting, hard linked
// const PTE_PER_PAGE: usize = PAGE_SIZE / 8;
// #[link_section = ".data.prepage"]
// pub(crate) static PAGE_TABLE: [usize; PTE_PER_PAGE] = [0; PTE_PER_PAGE];

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
#[allow(named_asm_labels)]
pub unsafe extern "C" fn _entry() -> ! {
    asm!(
        "
            ori         $t0, $zero, 0x1     # CSR_DMW1_PLV0
            lu52i.d     $t0, $t0, -2048     # UC, PLV0, 0x8000 xxxx xxxx xxxx
            csrwr       $t0, 0x180          # LOONGARCH_CSR_DMWIN0
            ori         $t0, $zero, 0x11    # CSR_DMW1_MAT | CSR_DMW1_PLV0
            lu52i.d     $t0, $t0, -1792     # CA, PLV0, 0x9000 xxxx xxxx xxxx
            csrwr       $t0, 0x181          # LOONGARCH_CSR_DMWIN1

            # Goto 1 if hart is not 0
            csrrd       $t1, 0x20       # read cpu from csr
            bnez        $t1, 1f

            # Enable PG 
            li.w		$t0, 0xb0		# PLV=0, IE=0, PG=1
            csrwr		$t0, 0x0        # LOONGARCH_CSR_CRMD
            li.w		$t0, 0x00		# PLV=0, PIE=0, PWE=0
            csrwr		$t0, 0x1        # LOONGARCH_CSR_PRMD
            li.w		$t0, 0x00		# FPE=0, SXE=0, ASXE=0, BTE=0
            csrwr		$t0, 0x2        # LOONGARCH_CSR_EUEN


            la.global   $sp, {boot_stack}
            li.d        $t0, {boot_stack_size}
            add.d       $sp, $sp, $t0       # setup boot stack
            csrrd       $a0, 0x20           # cpuid
            la.global   $t0, {entry}
            jirl        $zero,$t0,0

        1:
            li.w        $s0, {MBUF0}
            iocsrrd.d   $t0, $s0
            la.global   $t1, {sec_entry}
            bne         $t0, $t1, 1b
            jirl        $zero, $t1, 0
        ",
        boot_stack = sym BOOT_STACK,
        boot_stack_size = const KERNEL_STACK_SIZE,
        entry = sym _boot_hart_init,
        sec_entry = sym _entry_other_hart,
        MBUF0 = const LOONGARCH_CSR_MAIL_BUF0,
        options(noreturn)
    )
}

#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe extern "C" fn _entry_other_hart() -> ! {
    core::arch::asm!(
        "
        ori          $t0, $zero, 0x1     # CSR_DMW1_PLV0
        lu52i.d      $t0, $t0, -2048     # UC, PLV0, 0x8000 xxxx xxxx xxxx
        csrwr        $t0, 0x180          # LOONGARCH_CSR_DMWIN0
        ori          $t0, $zero, 0x11    # CSR_DMW1_MAT | CSR_DMW1_PLV0
        lu52i.d      $t0, $t0, -1792     # CA, PLV0, 0x9000 xxxx xxxx xxxx
        csrwr        $t0, 0x181          # LOONGARCH_CSR_DMWIN1

        li.w         $t0, {MBUF1}
        iocsrrd.d    $sp, $t0

        csrrd        $a0, 0x20                  # cpuid
        la.global    $t0, {entry}

        jirl $zero,$t0,0
        ",
        options(noreturn),
        MBUF1 = const loongArch64::consts::LOONGARCH_CSR_MAIL_BUF1,
        entry = sym _other_hart_init,
    )
}
