use arch::{Arch, ArchBoot, ArchInt, ArchSbi, _entry_other_hart, consts::KERNEL_ADDR_OFFSET};

use crate::{
    config::cpu::CPU_NUM,
    constant::banner::NOAXIOM_BANNER,
    cpu::get_hartid,
    driver,
    entry::init_proc::schedule_spawn_initproc,
    fs::fs_init,
    mm::{
        bss::bss_init,
        frame::frame_init,
        heap::heap_init,
        memory_set::{kernel_space_activate, kernel_space_init},
    },
    rust_main,
    sched::utils::block_on,
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
            Arch::hart_start(i, entry);
        }
    }
}

#[no_mangle]
pub extern "C" fn _other_hart_init(hart_id: usize, dtb: usize) -> ! {
    Arch::arch_init();
    kernel_space_activate();
    // register_to_hart(); // todo: add multipule devices interrupt support
    info!(
        "[other_init] entry init hart_id: {}, dtb_addr: {:#x}",
        hart_id, dtb as usize,
    );
    rust_main()
}

// pub static BOOT_HART_ID: AtomicUsize = AtomicUsize::new(0);

// TODO: dtb, init_proc
/// init bss, mm, console, and other drivers, then jump to rust_main,
/// called by [`super::boot`]
#[no_mangle]
pub extern "C" fn _boot_hart_init(_hartid: usize, dtb: usize) -> ! {
    // data init
    bss_init();
    heap_init();

    // log init
    Arch::arch_init();
    driver::log_init();

    // kernel space init
    frame_init();
    kernel_space_init();

    // device init
    driver::init(platform::get_dtb(dtb));

    // fs init
    Arch::enable_interrupt();
    block_on(fs_init());

    // spawn init_proc and wake other harts
    // crate::entry::init_proc::schedule_spawn_all_apps();
    schedule_spawn_initproc();
    wake_other_hart(get_hartid());

    // main
    println!("{}", NOAXIOM_BANNER);
    info!(
        "[first_init] entry init hart_id: {}, dtb_addr: {:#x}",
        get_hartid(),
        dtb as usize,
    );

    rust_main()
}
