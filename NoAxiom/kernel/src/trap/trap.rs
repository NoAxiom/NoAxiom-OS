use alloc::sync::Arc;
use core::arch::global_asm;

use riscv::register::{
    sstatus,
    stvec::{self, TrapMode},
};

use super::context::TrapContext;
use crate::{
    arch::interrupt::{enable_external_interrupt, enable_stimer_interrupt, is_interrupt_enabled},
    println,
    task::Task,
};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn user_trapvec();
    fn user_trapret(cx: *mut TrapContext);
    fn trap_from_kernel();
}

/// set trap entry in supervisor mode
pub fn set_kernel_trap_entry() {
    unsafe { stvec::write(trap_from_kernel as usize, TrapMode::Direct) }
}

/// set trap entry in user mode
pub fn set_user_trap_entry() {
    unsafe { stvec::write(user_trapvec as usize, TrapMode::Direct) }
}

/// trap init of current hart
pub fn trap_init() {
    set_kernel_trap_entry();
    assert!(
        !is_interrupt_enabled(),
        "kernel don't support global interrupt"
    );
    // disable_global_interrupt();
    enable_external_interrupt();
    enable_stimer_interrupt();
}

#[no_mangle]
/// kernel back to user
pub fn trap_restore(task: &Arc<Task>) {
    set_user_trap_entry();
    let cx = task.trap_context_mut();
    info!("trap_restore: sepc {:#x}", cx.sepc);
    info!("trap_restore: sp {:#x}", cx.user_reg[2]);
    task._debug_prio("task_main trap_restore begin2");
    warn!("trap_restore!!!");
    // kernel -> user
    unsafe {
        user_trapret(task.trap_context_mut());
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
