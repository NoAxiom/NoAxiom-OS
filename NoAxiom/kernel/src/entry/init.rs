use arch::{consts::KERNEL_ADDR_OFFSET, Arch, ArchBoot, ArchInfo, ArchInt, _entry_other_hart};
use driver::probe::probe_device;
use memory::bss::bss_init;
use platform::memory::VALID_PHYS_CPU_MASK;

use crate::{
    config::cpu::CPU_NUM,
    constant::banner::NOAXIOM_BANNER,
    cpu::get_hartid,
    entry::{init_proc::schedule_spawn_with_path, main::boot_broadcast},
    mm::{
        frame_init,
        heap::heap_init,
        memory_set::{kernel_space_activate, kernel_space_init},
    },
    sched::utils::block_on,
    time::clock::ktime_init,
    utils::log::log_init,
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
        if i != forbid_hart_id && (1 << i) & VALID_PHYS_CPU_MASK != 0 {
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
    crate::no_axiom_main()
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
    println!(
        "[kernel] SINGLECORE: CPU_NUM = {}, BOOT_HART = {}",
        CPU_NUM,
        get_hartid()
    );
    println!("[kernel] ARCH = {}", Arch::ARCH_NAME);
}

#[no_mangle]
pub extern "C" fn _boot_hart_init(_: usize, dtb: usize) -> ! {
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
    probe_device(dtb);

    // fs init
    with_interrupt_on!(block_on(crate::fs::init()));

    // spawn init_proc and wake other harts
    ktime_init();
    schedule_spawn_with_path();
    #[cfg(feature = "multicore")]
    wake_other_hart(get_hartid());

    // start task runner
    boot_broadcast();
    crate::no_axiom_main()
}
