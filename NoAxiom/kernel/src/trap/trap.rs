use alloc::sync::Arc;
use core::arch::global_asm;

use arch::interrupt::{
    enable_external_interrupt, enable_global_interrupt, enable_software_interrupt,
    enable_stimer_interrupt, is_interrupt_enabled,
};
use riscv::register::{
    sstatus,
    stvec::{self, TrapMode},
};

use super::context::TrapContext;
use crate::{task::Task, utils::current_pc};

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
    enable_global_interrupt();
    enable_software_interrupt();
    enable_stimer_interrupt();
}

#[no_mangle]
/// kernel back to user
pub fn trap_restore(task: &Arc<Task>) {
    // FIXME: disable interrupt before restore
    set_user_trap_entry();
    let cx = task.trap_context();
    trace!("[trap_restore] cx: {:?}", cx);
    trace!(
        "[trap_restore] tid {}, sepc {:#x}, sp {:#x}, argc {:#x}, argv {:#x}, envp {:#x}",
        task.tid(),
        cx.sepc,
        cx.user_reg[2],
        cx.user_reg[10],
        cx.user_reg[11],
        cx.user_reg[12],
    );
    // kernel -> user
    unsafe {
        user_trapret(task.trap_context_mut());
    }
    trace!(
        "[trap_restore] back to kernel, current_pc: {:#x}, inst: {:#x}",
        current_pc(),
        unsafe { *(current_pc() as *const u32) },
    );
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
