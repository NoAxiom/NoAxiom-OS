//! panic for no_std

use core::panic::PanicInfo;

use arch::{Arch, ArchInfo, ArchTrap};
use memory::{frame::print_frame_info, heap::print_heap_info};

use crate::{
    cpu::{current_cpu, get_hartid},
    syscall::{utils::current_syscall, Syscall},
    time::gettime::get_time_ms,
};

lazy_static::lazy_static! {
    static ref PANIC_FLAG: spin::Mutex<bool> = spin::Mutex::new(false);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!(
        "[PANIC] HART{}, TID{}, PANIC at {}ms, epc={:#x}, trap_depth={}",
        get_hartid(),
        current_cpu()
            .task
            .as_ref()
            .map_or_else(|| 0, |task| task.tid()),
        get_time_ms(),
        Arch::read_epc(),
        current_cpu().trap_depth(),
    );
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
    print_frame_info();
    print_heap_info();
    Syscall::sys_systemshutdown()
}
