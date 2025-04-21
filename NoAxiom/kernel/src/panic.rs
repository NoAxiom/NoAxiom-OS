//! panic for no_std

use core::panic::PanicInfo;

use arch::{Arch, ArchBoot};

use crate::cpu::{current_cpu, get_hartid};

lazy_static::lazy_static! {
    static ref PANIC_FLAG: spin::Mutex<bool> = spin::Mutex::new(false);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[PANIC] kernel triggered panic!!!");
    #[cfg(feature = "debug_sig")]
    println!(
        "[PANIC] during syscall {:?}",
        crate::syscall::syscall::CURRENT_SYSCALL.lock()
    );
    if let Some(task) = current_cpu().task.as_ref() {
        let cx = task.trap_context();
        info!("[PANIC] cx detected: {:#x?}", cx);
    }
    Arch::arch_info_print();
    if let Some(location) = info.location() {
        println!(
            "[PANIC] panicked at {}:{}\n[PANIC] HART{}, {}",
            location.file(),
            location.line(),
            get_hartid(),
            info.message().unwrap(),
        );
    } else {
        println!(
            "[PANIC (no_detail), HART{}] panicked: {}",
            get_hartid(),
            info.message().unwrap()
        );
    }
    loop {}
    platform::shutdown()
}
