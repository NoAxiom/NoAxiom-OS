use core::arch::asm;

use config::{
    cpu::CPU_NUM,
    mm::{KERNEL_STACK_SIZE, KERNEL_STACK_WIDTH, PAGE_SIZE},
};
use sbi_rt::hart_start;

use super::{context::freg_init, trap::trap_init, RV64};
use crate::{rv64::memory::KERNEL_ADDR_OFFSET, ArchBoot};

/// temp stack for kernel booting
#[link_section = ".bss.kstack"]
static BOOT_STACK: [u8; KERNEL_STACK_SIZE * CPU_NUM] = [0; KERNEL_STACK_SIZE * CPU_NUM];

const PTE_PER_PAGE: usize = PAGE_SIZE / 8;
/// temp page table for kernel booting, hard linked
#[link_section = ".data.prepage"]
static PAGE_TABLE: [usize; PTE_PER_PAGE] = {
    let mut arr: [usize; PTE_PER_PAGE] = [0; PTE_PER_PAGE];
    // create pte with all flags set
    macro_rules! page_table_config {
        ($($id:expr, $addr:expr)*) => {
            $(arr[$id] = ($addr << 10) | 0xcf;)*
        };
    }
    // va: 0x0000_0000_8000_0000 -> pa: 0x0000_0000_8000_0000
    // va: 0xffff_fc00_8000_0000 -> pa: 0x0000_0000_8000_0000
    page_table_config! {
        // 0, 0x00000
        // 1, 0x40000
        2, 0x80000
        // 3, 0xc0000
        // 0x100, 0x00000
        // 0x101, 0x40000
        0x102, 0x80000
        // 0x103, 0xc0000
    };
    arr
};

extern "C" {
    fn _boot_hart_init();
    fn _other_hart_init();
}

/// the entry of whole kernel
#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe extern "C" fn _entry() -> ! {
    asm!("
            mv      tp, a0

            mv      gp, a1
            add     t0, a0, 1
            slli    t0, t0, {kernel_stack_shift}
            la      sp, {boot_stack}
            add     sp, sp, t0

            li      s0, {kernel_addr_offset}
            or      sp, sp, s0

            // activate page table
            la      t0, {page_table}
            srli    t0, t0, 12
            li      t1, 8 << 60
            or      t0, t0, t1
            csrw    satp, t0
            sfence.vma

            li      t1, {kernel_addr_offset}
            or      gp, gp, t1

            mv      a1, gp
            la      t0, {entry}
            or      t0, t0, t1
            mv      a0, tp
            jalr    t0
        ",
        page_table = sym PAGE_TABLE,
        boot_stack = sym BOOT_STACK,
        kernel_addr_offset = const KERNEL_ADDR_OFFSET,
        kernel_stack_shift = const KERNEL_STACK_WIDTH,
        entry = sym _boot_hart_init,
        options(noreturn),
    )
}

#[naked]
#[no_mangle]
pub unsafe extern "C" fn _entry_other_hart() -> ! {
    asm!("
            mv      tp, a0

            mv      gp, a1
            add     t0, a0, 1
            slli    t0, t0, {kernel_stack_shift}
            la      sp, {boot_stack}
            add     sp, sp, t0

            li      s0, {kernel_addr_offset}
            or      sp, sp, s0

            // activate page table
            la      t0, {page_table}
            srli    t0, t0, 12
            li      t1, 8 << 60
            or      t0, t0, t1
            csrw    satp, t0
            sfence.vma

            li      t1, {kernel_addr_offset}
            or      gp, gp, t1

            mv      a1, gp
            la      t0, {entry}
            or      t0, t0, t1
            mv      a0, tp
            jalr    t0
        ",
        page_table = sym PAGE_TABLE,
        boot_stack = sym BOOT_STACK,
        kernel_addr_offset = const KERNEL_ADDR_OFFSET,
        kernel_stack_shift = const KERNEL_STACK_WIDTH,
        entry = sym _other_hart_init,
        options(noreturn),
    )
}

impl ArchBoot for RV64 {
    fn arch_init() {
        trap_init();
        freg_init();
    }
    // hart start
    fn hart_start(hartid: usize, start_addr: usize) {
        let x = hart_start(hartid, start_addr, 0);
        if x.is_err() {
            panic!("hart_start failed");
        }
    }
}
