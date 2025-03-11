use core::sync::atomic::AtomicUsize;

use arch::{Arch, ArchInt, ArchSbi};

use crate::{
    config::{arch::CPU_NUM, mm::KERNEL_ADDR_OFFSET},
    constant::banner::NOAXIOM_BANNER,
    cpu::get_hartid,
    device::init::device_init,
    driver::log::log_init,
    entry::{boot::_entry_other_hart, init_proc::schedule_spawn_initproc},
    fs::fs_init,
    mm::{bss::bss_init, frame::frame_init, hart_mm_init, heap::heap_init},
    platform::{
        base_riscv::platforminfo::platform_info_from_dtb,
        platform_init,
        plic::{init_plic, register_to_hart},
    },
    rust_main,
    sched::utils::block_on,
    trap::trap::trap_init,
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
            Arch::hart_start(i, entry, 0);
            // let result = hart_start(i, entry, 0);
            // if result.error == 0 {
            //     info!("[awake_other_hart] hart {:x} start successfully", i);
            // } else {
            //     error!(
            //         "[awake_other_hart] error when waking {}, error code:
            // {:?}",         i, result
            //     );
            // }
        }
    }
}

#[no_mangle]
pub fn other_hart_init(hart_id: usize, dtb: usize) {
    Arch::enable_user_memory_access();
    hart_mm_init();
    trap_init();
    // register_to_hart(); // todo: add multipule devices interrupt support
    info!(
        "[other_init] entry init hart_id: {}, dtb_addr: {:#x}",
        hart_id, dtb as usize,
    );
    rust_main();
    unreachable!();
}

pub static BOOT_HART_ID: AtomicUsize = AtomicUsize::new(0);

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
    BOOT_HART_ID.store(get_hartid(), core::sync::atomic::Ordering::SeqCst);
    Arch::enable_user_memory_access();

    // hart resources init
    hart_mm_init();
    trap_init();

    // global resources: fs init
    let platfrom_info = platform_info_from_dtb(dtb);
    platform_init(get_hartid(), dtb);
    init_plic(platfrom_info.plic.start + KERNEL_ADDR_OFFSET);
    device_init();
    register_to_hart();

    block_on(fs_init());

    // spawn init_proc and wake other harts
    // entry::init_proc::schedule_spawn_all_apps();
    schedule_spawn_initproc();
    wake_other_hart(get_hartid());

    // main
    println!("{}", NOAXIOM_BANNER);
    debug_print();
    info!(
        "[first_init] entry init hart_id: {}, dtb_addr: {:#x}",
        get_hartid(),
        dtb as usize,
    );
    rust_main();
    unreachable!();
}

pub fn debug_print() {
    #[cfg(feature = "debug")]
    {
        #[cfg(feature = "async_fs")]
        warn!("[compile_args] async-fs is on");
        #[cfg(feature = "multicore")]
        warn!("[compile_args] multicore is on");
    }
    #[cfg(not(feature = "debug"))]
    {
        info!("[compile_args] debug is off\n");
    }
}
