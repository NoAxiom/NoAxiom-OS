use core::arch::asm;

use crate::{
    arch::interrupt::enable_user_memory_access,
    config::{arch::CPU_NUM, mm::KERNEL_PHYS_ENTRY},
    constant::banner::NOAXIOM_BANNER,
    cpu::get_hartid,
    driver::{log::log_init, sbi::hart_start},
    mm::mm_init,
    println, rust_main,
    sched::schedule_spawn_new_process,
    task::load_app::app_nums,
    trap::trap_init,
};

/// This function is called only once during booting.
/// DO NOT try to modify any global / unstacked variable before this function!
/// NOTE THAT this function will not clear any data on the kernel stack,
/// since the beginning address is `ekstack`.
fn bss_init() {
    extern "C" {
        fn ekstack();
        fn ebss();
    }
    (ekstack as usize..ebss as usize).for_each(|x| unsafe { (x as *mut u8).write_volatile(0) });
}

fn global_resources_init() {
    bss_init();
    log_init();
    mm_init();
}

fn hart_resources_init() {
    trap_init();
}

/// awake other core
#[allow(unused)]
fn awake_other_hart(forbid_hart_id: usize) {
    info!("awake_other_hart, forbid hart: {}", forbid_hart_id);
    for i in 0..CPU_NUM {
        if i != forbid_hart_id {
            let result = hart_start(i, KERNEL_PHYS_ENTRY, 0);
            if result.error == 0 {
                info!("[awake_other_hart] hart {:x} start successfully", i);
            } else {
                error!(
                    "[awake_other_hart] error when waking {}, error code: {:?}",
                    i,
                    result.get_sbi_error()
                );
            }
        }
    }
}

/// spawn all apps, only used in debug
#[allow(unused)]
fn schedule_spawn_all_apps() {
    for i in 0..app_nums() {
        info!("[init] spawn app_{}", i);
        schedule_spawn_new_process(i);
    }
}

/// BOOT_FLAG is used to ensure that the kernel is only initialized once
/// and that the kernel is not re-entered after the first initialization.
/// SAFETY: it is supposed not to use AtomicBool since it may damage the heap
/// space.
static mut BOOT_FLAG: bool = false;

// TODO: dtb, init_proc
/// init bss, mm, console, and other drivers, then jump to rust_main,
/// called by `super::boot`
#[no_mangle]
pub fn boot_hart_init(_: usize, dtb: usize) {
    if unsafe { !BOOT_FLAG } {
        enable_user_memory_access();
        // WARNING: don't try to modify any global variable before this line
        // because it will be overwritten by clear_bss
        global_resources_init();
        unsafe {
            BOOT_FLAG = true;
            asm!("fence rw, rw");
        }
        println!("{}", NOAXIOM_BANNER);
        info!(
            "[init] entry init hart_id: {}, dtb_addr: {:#x}",
            get_hartid(),
            dtb as usize,
        );
        schedule_spawn_all_apps();
        awake_other_hart(get_hartid());
        hart_resources_init();
        rust_main();
    } else {
        enable_user_memory_access();
        hart_resources_init();
        rust_main();
    }
}
