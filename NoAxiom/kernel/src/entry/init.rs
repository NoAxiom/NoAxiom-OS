use core::sync::atomic::{AtomicBool, Ordering};

use crate::{
    arch::interrupt::enable_user_memory_access, cpu::hartid, println,
    sched::schedule_spawn_new_process,
};

fn pre_init() {
    enable_user_memory_access();
}

fn global_resources_init() {
    crate::mm::bss::bss_init();
    crate::driver::log::log_init();
    crate::mm::mm_init();
}

fn hart_resources_init() {
    crate::trap::trap_init();
}

static mut BOOT_FLAG: AtomicBool = AtomicBool::new(false);
static mut INIT_FLAG: AtomicBool = AtomicBool::new(false);

// TODO: dtb
/// init bss, mm, console, and other drivers, then jump to rust_main,
/// called by [`super::boot`]
#[no_mangle]
pub(crate) fn init(_hart_id: usize, _dtb: usize) {
    pre_init();
    if unsafe {
        BOOT_FLAG
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    } {
        global_resources_init();
        println!("[entry] entry init hart_id: {}", hartid());
        println!("{}", crate::constant::banner::NOAXIOM_BANNER);
        // TODO: spawn init_proc
        schedule_spawn_new_process(0);
        schedule_spawn_new_process(1);
        unsafe {
            INIT_FLAG.store(true, Ordering::SeqCst);
        }
    } else {
        while unsafe { !INIT_FLAG.load(Ordering::SeqCst) } {}
    }
    hart_resources_init();
    crate::rust_main();
}
