use core::{arch::asm, ptr};

use config::{arch::CPU_NUM, mm::KERNEL_STACK_SIZE};
use log::info;
use loongArch64::{
    consts::LOONGARCH_CSR_MAIL_BUF0,
    cpu::{get_palen, get_valen},
    register::{crmd, dmw0, dmw1, pgd, pgdh, pgdl, prcfg1, prcfg2, prcfg3, pwch, pwcl},
};

use super::{context::freg_init, memory::tlb_init, time::time_init, trap::trap_init, LA64};
use crate::{ArchAsm, ArchBoot};

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
            csrrd       $a0, 0x20           # a0 = cpuid
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

        csrrd        $a0, 0x20                  # a0 = cpuid
        la.global    $t0, {entry}

        jirl $zero,$t0,0
        ",
        options(noreturn),
        MBUF1 = const loongArch64::consts::LOONGARCH_CSR_MAIL_BUF1,
        entry = sym _other_hart_init,
    )
}

impl ArchBoot for LA64 {
    fn arch_init() {
        freg_init();
        trap_init();
        time_init();
        tlb_init();
    }
    fn arch_info_print() {
        let hartid = LA64::get_hartid();
        info!("[LA64] arch: loongarch64");
        info!("[LA64] hart id: {}", hartid);
        let boot_start_top_0 = ptr::from_ref(&BOOT_STACK) as usize;
        let boot_start_top_cur = boot_start_top_0 + KERNEL_STACK_SIZE * hartid;
        info!(
            "[LA64] kernel_stack addr: [{:#x}, {:#x})",
            boot_start_top_cur,
            boot_start_top_cur + KERNEL_STACK_SIZE
        );
        info!(
            "[LA64] max_valen: {}, max_palen: {}",
            get_valen(),
            get_palen()
        );
        info!("[LA64] crmd: {:?}", crmd::read(),);
        info!("[LA64] dmw0: {:?}", dmw0::read());
        info!("[LA64] dmw1: {:?}", dmw1::read());
        info!("[LA64] prcfg1: {:?}", prcfg1::read());
        info!("[LA64] prcfg2: {:#x}", prcfg2::read().raw());
        info!("[LA64] prcfg3: {:#x}", prcfg3::read().raw());

        let pwcl = pwcl::read();
        let pwch = pwch::read();
        let info = [
            (pwcl.ptbase(), pwcl.ptwidth()),
            (pwcl.dir1_base(), pwcl.dir1_width()),
            (pwcl.dir2_base(), pwcl.dir2_width()),
            (pwch.dir3_base(), pwch.dir3_width()),
            (pwch.dir4_base(), pwch.dir4_width()),
        ];
        info!("[LA64] pwcl: {:#x}", pwcl::read().raw());
        info!("[LA64] pwch: {:#x}", pwch::read().raw());
        for i in 0..5 {
            info!("[LA64] pwc[{}]: {:?}", i, info[i]);
        }

        info!("[LA64] pgd: {:?}", pgd::read());
        info!("[LA64] pgdl: {:?}", pgdl::read());
        info!("[LA64] pgdh: {:?}", pgdh::read());

        let save: usize;
        unsafe { asm!("csrrd {}, 0x30", out(reg) save) };
        info!("[LA64] SAVE: {:#x}", save);
    }
}
