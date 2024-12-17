use crate::{
    arch::interrupt::{
        enable_external_interrupt, enable_global_interrupt, enable_user_memory_access,
        is_interrupt_enabled,
    },
    config::{arch::CPU_NUM, mm::KERNEL_ADDR_OFFSET},
    constant::banner::NOAXIOM_BANNER,
    cpu::get_hartid,
    device::device_init,
    driver::{log::log_init, sbi::hart_start},
    entry::boot::_entry_other_hart,
    fs::fs_init,
    mm::{self, frame::frame_init, heap::heap_init},
    platform::{self, base_riscv::platforminfo::platform_info_from_dtb, plic::init_plic},
    println, rust_main,
    sched::{schedule_spawn_new_ktask, schedule_spawn_new_process},
    task::load_app::app_nums,
    trap::set_kernel_trap_entry,
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

/// awake other core
#[allow(unused)]
fn awake_other_hart(forbid_hart_id: usize) {
    let entry = (_entry_other_hart as usize) & (!KERNEL_ADDR_OFFSET);
    info!(
        "awake_other_hart, forbid hart: {}, entry: {:#x}",
        forbid_hart_id, entry
    );
    for i in 0..CPU_NUM {
        if i != forbid_hart_id {
            let result = hart_start(i, entry, 0);
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
    schedule_spawn_new_ktask(fs_init(), 0);

    for i in 0..app_nums() {
        info!("[init] spawn app_{}", i);
        schedule_spawn_new_process(i);
    }
}

/// spawn init process
#[allow(unused)]
fn spawn_initproc() {
    info!("[init] spawn initproc");
    schedule_spawn_new_process(0);
}

// TODO: dtb, init_proc
/// init bss, mm, console, and other drivers, then jump to rust_main,
/// called by `super::boot`
#[no_mangle]
pub fn boot_hart_init(hart_id: usize, dtb: usize) {
    // WARNING: don't try to modify any global variable before this line
    // because it will be overwritten by clear_bss
    bss_init();
    heap_init();
    log_init();
    frame_init();
    enable_user_memory_access();
    schedule_spawn_all_apps();
    mm::hart_mm_init();
    let platfrom_info = platform_info_from_dtb(dtb);
    platform::init(hart_id, dtb);
    init_plic(platfrom_info.plic.start + KERNEL_ADDR_OFFSET);
    device_init();
    // WARNING: all global variables should be initialized before this line
    println!("{}", NOAXIOM_BANNER);
    info!(
        "[first_init] entry init hart_id: {}, dtb_addr: {:#x}",
        hart_id, dtb as usize,
    );
    awake_other_hart(get_hartid());
    rust_main();
}

#[no_mangle]
pub fn other_hart_init(hart_id: usize, dtb: usize) {
    enable_user_memory_access();
    mm::hart_mm_init();
    info!(
        "[other_init] entry init hart_id: {}, dtb_addr: {:#x}",
        hart_id, dtb as usize,
    );
    rust_main();
}
