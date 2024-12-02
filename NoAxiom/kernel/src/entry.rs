use core::arch::asm;

use crate::{
    arch::interrupt::enable_visit_user_memory,
    config::{
        arch::CPU_NUM,
        mm::{BOOT_STACK_SIZE, BOOT_STACK_WIDTH, KERNEL_ADDR_OFFSET, PTE_PER_PAGE},
    },
    driver::sbi::hart_start,
    mm::pte::PageTableEntry,
    println, rust_main,
};

/// temp stack for kernel booting
#[link_section = ".bss.stack"]
static BOOT_STACK: [u8; BOOT_STACK_SIZE * CPU_NUM] = [0; BOOT_STACK_SIZE * CPU_NUM];

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
    page_table_config! {
        1, 0x40000
        2, 0x80000
        0x100, 0x00000
        0x101, 0x40000
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
            slli    t0, t0, {kernel_stack_size}
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
            jalr    t0
        ",
        page_table = sym PAGE_TABLE,
        boot_stack = sym BOOT_STACK,
        kernel_addr_offset = const KERNEL_ADDR_OFFSET,
        kernel_stack_size = const BOOT_STACK_WIDTH,
        entry = sym init,
        options(noreturn),
    )
}

// TODO: dtb
/// init bss, mm, console, and other drivers,
/// then jump to rust_main
#[no_mangle]
pub(crate) fn init(hart_id: usize, _dtb: usize) {
    // if hart_id == 0 {
    crate::mm::bss::bss_init();
    crate::driver::log::log_init();
    crate::mm::mm_init();
    // }
    println!("[entry] first init hart_id: {}", hart_id);

    // let platform_info = platform_info_from_dtb(dtb);
    // platform::init(hart_id, dtb);
    // interrupt::init_plic(platform_info.plic.start + KERNEL_ADDR_OFFSET);

    #[cfg(feature = "multicore")]
    init_other_hart(hart_id);

    enable_visit_user_memory();
    rust_main(hart_id);
}

#[naked]
#[no_mangle]
pub(crate) unsafe extern "C" fn other_hart_entry() {
    asm!("
            mv      tp, a0
            mv      t0, a0
            slli    t0, t0, {kernel_stack_size}
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

            la      t0, {entry}
            or      t0, t0, s0
            jalr    t0
        ",
        boot_stack = sym BOOT_STACK,
        page_table = sym PAGE_TABLE,
        kernel_addr_offset = const KERNEL_ADDR_OFFSET,
        kernel_stack_size = const BOOT_STACK_WIDTH,
        entry = sym call_other_hart_main,
        options(noreturn),
    )
}

#[no_mangle]
pub(crate) extern "C" fn call_other_hart_main(hart_id: usize, _dtb: usize) {
    enable_visit_user_memory();
    rust_main(hart_id);
}

/// awake other core
#[allow(unused)]
pub fn init_other_hart(forbid_hart_id: usize) {
    let aux_core_func = (other_hart_entry as usize) & (!KERNEL_ADDR_OFFSET);

    let start_id = 0;
    // there's no need to wake hart 0 on vf2 platform
    #[cfg(feature = "vf2")]
    let start_id = 1;

    println!("init_other_hart, previous hart: {}", forbid_hart_id);
    for i in start_id..CPU_NUM {
        if i != forbid_hart_id {
            println!("[init_other_hart] secondary addr: {:#x}", aux_core_func);

            let res = hart_start(i, aux_core_func, 0);
            if res.error == 0 {
                println!("[init_other_hart] hart {:x} start successfully", i);
            }
        }
    }
}
