use alloc::sync::Arc;
use core::arch::global_asm;

use riscv::register::{
    scause, sstatus, stval,
    stvec::{self, TrapMode},
};

use crate::{
    config::mm::{TRAMPOLINE, TRAP_CONTEXT_BASE},
    println,
    task::Task,
};

global_asm!(include_str!("./trap.S"));

/// Initialize trap handling
pub fn init() {
    set_kernel_trap_entry();
}

pub fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

#[no_mangle]
/// kernel back to user
pub fn trap_restore(task: &Arc<Task>) {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT_BASE;
    let user_satp = task.token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    info!(
        "[kernel] trap_return: ..before return, task-token: {:#x}  {}",
        task.token(),
        task.token()
    );
    unsafe {
        core::arch::asm!(
            "fence.i",
            "jr {restore_va}",         // jump to new addr of __restore asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = virt addr of Trap Context
            in("a1") user_satp,        // a1 = phy addr of usr page table
            options(noreturn)
        );
    }
}

/// debug: show sstatus
#[allow(unused)]
pub fn show_sstatus() {
    println!("show sstatus");
    let sstatus = sstatus::read();
    let spie = sstatus.spie(); // previous sie value
    let sie = sstatus.sie();
    println!("spie:{:?}", spie);
    println!("sie:{:?}", sie);
}

pub fn trap_from_kernel() -> ! {
    use riscv::register::sepc;
    info!("stval = {:#x}, sepc = {:#x}", stval::read(), sepc::read());
    panic!("a trap {:?} from kernel!", scause::read().cause());
}
