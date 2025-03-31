use core::panic;

use arch::{Arch, ArchInt, ArchMemory, ArchSbi, ArchTrap, _entry_other_hart};

use crate::{
    config::{arch::CPU_NUM, mm::KERNEL_ADDR_OFFSET},
    constant::banner::NOAXIOM_BANNER,
    cpu::get_hartid,
    device::init::device_init,
    driver::log::log_init,
    entry::init_proc::schedule_spawn_initproc,
    fs::fs_init,
    mm::{
        bss::bss_init,
        frame::frame_init,
        heap::heap_init,
        memory_set::{kernel_space_activate, kernel_space_init},
    },
    platform::{
        base_riscv::platforminfo::platform_info_from_dtb,
        platform_init,
        plic::{init_plic, register_to_hart},
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
pub extern "C" fn _other_hart_init(hart_id: usize, dtb: usize) {
    Arch::trap_init();
    Arch::tlb_init();
    kernel_space_activate();
    // register_to_hart(); // todo: add multipule devices interrupt support
    info!(
        "[other_init] entry init hart_id: {}, dtb_addr: {:#x}",
        hart_id, dtb as usize,
    );
    rust_main();
    unreachable!();
}

// pub static BOOT_HART_ID: AtomicUsize = AtomicUsize::new(0);

// TODO: dtb, init_proc
/// init bss, mm, console, and other drivers, then jump to rust_main,
/// called by [`super::boot`]
#[no_mangle]
pub extern "C" fn _boot_hart_init(_: usize, dtb: usize) {
    // data init
    bss_init();
    heap_init();

    // log init
    Arch::trap_init();
    log_init();

    // kernel space init
    frame_init();
    Arch::tlb_init();
    kernel_space_init();

    Arch::enable_interrupt();

    #[cfg(target_arch = "loongarch64")]
    {
        /// QEMU Loongarch64 Virt Machine:
        /// https://github.com/qemu/qemu/blob/master/include/hw/loongarch/virt.h
        pub(crate) const QEMU_DTB_ADDR: usize = 0x100000;
        let dtb = (QEMU_DTB_ADDR | KERNEL_ADDR_OFFSET) as usize;
        unsafe {
            if fdt::Fdt::from_ptr((dtb) as *const u8).is_ok() {
                info!("Loongarch64 QEMU DTB: {:#x}", dtb);
            } else {
                panic!("Loongarch64 QEMU DTB: {:#x} is invalid", dtb);
            }
        }
    }

    // device init
    info!("[device init] dtb addr: {:#x}", dtb);
    let platfrom_info = platform_info_from_dtb(dtb);
    debug!("Platform Info: {:?}", platfrom_info);
    platform_init(get_hartid(), dtb);
    init_plic(platfrom_info.plic.start + KERNEL_ADDR_OFFSET);
    device_init();
    register_to_hart();

    // fs init
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

    rust_main();
    unreachable!();
}
