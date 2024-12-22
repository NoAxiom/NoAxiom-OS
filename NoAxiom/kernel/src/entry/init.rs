use crate::{
    arch::interrupt::enable_user_memory_access,
    config::{arch::CPU_NUM, mm::KERNEL_ADDR_OFFSET},
    constant::banner::NOAXIOM_BANNER,
    cpu::get_hartid,
    device::init::device_init,
    driver::{log::log_init, sbi::hart_start},
    entry::{
        boot::_entry_other_hart,
        init_proc::{schedule_spawn_all_apps, schedule_spawn_initproc},
    },
    fs::fs_init,
    mm::{bss::bss_init, frame::frame_init, hart_mm_init, heap::heap_init},
    platform::{
        base_riscv::platforminfo::platform_info_from_dtb,
        platform_init,
        plic::{init_plic, register_to_hart},
    },
    println, rust_main,
    sched::task::spawn_ktask,
    trap::trap_init,
};

/// awake other core
#[allow(unused)]
pub fn wake_other_hart(forbid_hart_id: usize) {
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

pub async fn async_init() {
    fs_init().await;
    // schedule_spawn_all_apps();
    wake_other_hart(get_hartid());
    info!("[kernel] async init done");
}

#[no_mangle]
pub fn other_hart_init(hart_id: usize, dtb: usize) {
    enable_user_memory_access();
    hart_mm_init();
    trap_init();
    register_to_hart();
    info!(
        "[other_init] entry init hart_id: {}, dtb_addr: {:#x}",
        hart_id, dtb as usize,
    );
    rust_main();
}

// TODO: dtb, init_proc
/// init bss, mm, console, and other drivers, then jump to rust_main,
/// called by [`super::boot`]
#[no_mangle]
pub fn boot_hart_init(_: usize, dtb: usize) {
    // global resources init
    bss_init();
    heap_init();
    log_init();
    frame_init();
    enable_user_memory_access();

    // hart resources init
    hart_mm_init();
    trap_init();

    // global resources: platform & device init
    let platfrom_info = platform_info_from_dtb(dtb);
    platform_init(get_hartid(), dtb);
    init_plic(platfrom_info.plic.start + KERNEL_ADDR_OFFSET);
    device_init();
    register_to_hart();

    // main
    spawn_ktask(async_init());
    println!("{}", NOAXIOM_BANNER);
    info!(
        "[first_init] entry init hart_id: {}, dtb_addr: {:#x}",
        get_hartid(),
        dtb as usize,
    );
    // schedule_spawn_initproc();
    schedule_spawn_all_apps();
    rust_main();
}
