use core::{
    arch::asm,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    arch::interrupt::enable_visit_user_memory,
    config::{
        arch::CPU_NUM,
        mm::{
            BOOT_STACK_SIZE, BOOT_STACK_WIDTH, KERNEL_ADDR_OFFSET, KERNEL_PHYS_ENTRY, PTE_PER_PAGE,
        },
    },
    cpu::hartid,
    driver::sbi::hart_start,
    mm::pte::PageTableEntry,
    println, rust_main,
};

static mut BOOT_FLAG: AtomicBool = AtomicBool::new(false);
static mut INIT_FLAG: AtomicBool = AtomicBool::new(false);

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
            mv      a0, tp
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
pub(crate) fn init(_hart_id: usize, _dtb: usize) {
    if unsafe {
        BOOT_FLAG
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    } {
        crate::mm::bss::bss_init();
        crate::driver::log::log_init();
        crate::mm::mm_init();
        enable_visit_user_memory();
        println!("[entry] entry init hart_id: {}", hartid());
        println!("{}", crate::constant::banner::NOAXIOM_BANNER);
        crate::task::spawn_new_process(0);
        crate::task::spawn_new_process(1);
        crate::task::spawn_new_process(2);
        unsafe {
            INIT_FLAG.store(true, Ordering::SeqCst);
        }
        // init_other_hart(hart_id);
    } else {
        enable_visit_user_memory();
        while unsafe { !INIT_FLAG.load(Ordering::SeqCst) } {}
    }
    rust_main();
}

/// awake other core
#[allow(unused)]
pub fn init_other_hart(forbid_hart_id: usize) {
    // let aux_core_func = (other_hart_entry as usize) & (!KERNEL_ADDR_OFFSET);
    // println!("aux_core_func: {:#x}", aux_core_func);

    let start_id = 0;
    // there's no need to wake hart 0 on vf2 platform
    #[cfg(feature = "vf2")]
    let start_id = 1;

    info!("init_other_hart, forbid hart: {}", forbid_hart_id);
    for i in start_id..CPU_NUM {
        if i != forbid_hart_id {
            // info!("[init_other_hart] secondary addr: {:#x}", aux_core_func);
            let result = hart_start(i, KERNEL_PHYS_ENTRY, 0);
            if result.error != 0 {
                println!(
                    "[init_other_hart] error when waking {}, error code: {:?}",
                    i,
                    result.get_sbi_error()
                );
            }
            info!("[init_other_hart] hart {:x} start successfully", i);
        }
    }
}
