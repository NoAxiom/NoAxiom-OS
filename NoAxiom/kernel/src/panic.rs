//! panic for no_std

use core::panic::PanicInfo;

use arch::{Arch, ArchInfo};

use crate::{
    cpu::{current_cpu, get_hartid},
    syscall::utils::current_syscall,
    time::gettime::get_time_ms,
};

lazy_static::lazy_static! {
    static ref PANIC_FLAG: spin::Mutex<bool> = spin::Mutex::new(false);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!(
        "[PANIC] HART{}, TID{}, PANIC at {}ms",
        get_hartid(),
        current_cpu()
            .task
            .as_ref()
            .map_or_else(|| 0, |task| task.tid()),
        get_time_ms(),
    );
    #[cfg(feature = "debug_sig")]
    println!("[PANIC] during syscall {:?}", current_syscall());
    if let Some(task) = current_cpu().task.as_ref() {
        let cx = task.trap_context();
        println!("[PANIC] cx detected: {:#x?}", cx);
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
    println!("[PANIC] press any key to shutdown");
    while platform::getchar() as i8 == -1 {}
    platform::shutdown()
}
