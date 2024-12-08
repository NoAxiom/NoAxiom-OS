use core::arch::asm;

use crate::{
    arch::interrupt::enable_user_memory_access,
    config::{arch::CPU_NUM, mm::KERNEL_PHYS_ENTRY},
    constant::banner::NOAXIOM_BANNER,
    cpu::get_hartid,
    driver::sbi::hart_start,
    println, rust_main,
    sched::schedule_spawn_new_process,
};

fn global_resources_init() {
    crate::mm::bss::bss_init();
    crate::driver::log::log_init();
    crate::mm::mm_init();
}

fn hart_resources_init() {
    crate::trap::trap_init();
}

/// BOOT_FLAG is used to ensure that the kernel is only initialized once
/// and that the kernel is not re-entered after the first initialization
/// SAFETY: I suppose not to use AtomicBool since it may damage the heap space
static mut BOOT_FLAG: bool = false;

// TODO: dtb
/// init bss, mm, console, and other drivers, then jump to rust_main,
/// called by `super::boot`
#[no_mangle]
pub(crate) fn boot_hart_init(_: usize, __: usize) {
    if unsafe { !BOOT_FLAG } {
        enable_user_memory_access();
        // WARNING: don't try to modify any global variable before this line
        // because it will be overwritten by clear_bss
        global_resources_init();
        unsafe {
            BOOT_FLAG = true;
            asm!("fence rw, rw");
        }
        info!(
            "[init] entry init hart_id: {}, boot_flag: {}",
            get_hartid(),
            unsafe { BOOT_FLAG }
        );
        println!("{}", NOAXIOM_BANNER);
        // TODO: spawn init_proc
        for i in 0..crate::task::load_app::app_nums() {
            info!("[entry] spawn app_{}", i);
            schedule_spawn_new_process(i);
        }
        awake_other_hart(get_hartid());
        hart_resources_init();
        rust_main();
    } else {
        enable_user_memory_access();
        hart_resources_init();
        rust_main();
    }
}

/// awake other core
#[allow(unused)]
pub fn awake_other_hart(forbid_hart_id: usize) {
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
