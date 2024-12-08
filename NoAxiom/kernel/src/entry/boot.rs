use core::arch::asm;

use crate::{
    config::{arch::CPU_NUM, mm::*},
    mm::pte::PageTableEntry,
};

/// temp stack for kernel booting
#[link_section = ".bss.kstack"]
static BOOT_STACK: [u8; KERNEL_STACK_SIZE * CPU_NUM] = [0; KERNEL_STACK_SIZE * CPU_NUM];

/// temp page table for kernel booting, hard linked
#[link_section = ".data.prepage"]
static PAGE_TABLE: [PageTableEntry; PTE_PER_PAGE] = {
    let mut arr: [PageTableEntry; PTE_PER_PAGE] = [PageTableEntry(0); PTE_PER_PAGE];
    // create pte with all flags set
    macro_rules! page_table_config {
        ($($id:expr, $addr:expr)*) => {
            $(arr[$id] = PageTableEntry(($addr << 10) | 0xcf);)*
        };
    }
    // va: 0x0000_0000_8000_0000 -> pa: 0x0000_0000_8000_0000
    // va: 0xffff_fc00_8000_0000 -> pa: 0x0000_0000_8000_0000
    page_table_config! {
        2, 0x80000
        0x102, 0x80000
    };
    arr
};

/// the entry of whole kernel
#[naked]
#[no_mangle]
#[link_section = ".text.entry"]
unsafe extern "C" fn _entry() -> ! {
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
        entry = sym super::init::boot_hart_init,
        options(noreturn),
    )
}
