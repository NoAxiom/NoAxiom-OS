use arch::{Arch, ArchBoot, ArchInfo, ArchInt, _entry_other_hart, consts::KERNEL_ADDR_OFFSET};
use device::{device_init, dtb_init};

use crate::{
    config::cpu::CPU_NUM,
    constant::banner::NOAXIOM_BANNER,
    cpu::get_hartid,
    driver::log::log_init,
    entry::init_proc::schedule_spawn_with_path,
    mm::{
        bss::bss_init,
        frame::frame_init,
        heap::heap_init,
        memory_set::{kernel_space_activate, kernel_space_init},
    },
    sched::{runtime::run_tasks, utils::block_on},
    time::clock::ktime_init,
    with_interrupt_on,
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
    Arch::enable_interrupt();
    run_tasks()
}

fn hello_world() {
    println!("{}", NOAXIOM_BANNER);
    println!("\u{1B}[1;34m=============================================\u{1B}[0m\n");
    #[cfg(feature = "multicore")]
    println!(
        "[kernel] MULTICORE: CPU_NUM = {}, BOOT_HART = {}",
        CPU_NUM,
        get_hartid()
    );
    #[cfg(not(feature = "multicore"))]
    println!("[kernel] SINGLECORE: CPU_NUM = {}", CPU_NUM);
    println!("[kernel] ARCH = {}", Arch::ARCH_NAME);
}

#[no_mangle]
pub extern "C" fn _boot_hart_init(
    #[allow(unused)] arg0: usize,
    #[allow(unused)] arg1: usize,
    #[allow(unused)] arg2: usize,
    #[allow(unused)] arg3: usize,
) -> ! {
    // data init
    bss_init();
    heap_init();

    // log init
    Arch::arch_init();
    log_init();

    // print basic info
    hello_world();

    // kernel space init
    frame_init();
    kernel_space_init();

    // device init
    dtb_init(arg1);
    device_init();

    // fs init
    with_interrupt_on!(block_on(crate::fs::init()));

    // spawn init_proc and wake other harts
    ktime_init();
    schedule_spawn_with_path();
    wake_other_hart(get_hartid());

    // start task runner
    run_tasks()
}
