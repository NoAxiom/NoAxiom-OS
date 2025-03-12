use core::arch::global_asm;

use super::{context::TrapContext, register::set_trap_entry};
use crate::rv64::interrupt::{
    enable_external_interrupt, enable_global_interrupt, enable_software_interrupt,
    enable_stimer_interrupt,
};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn user_trapvec();
    fn user_trapret(cx: *mut TrapContext);
    fn trap_from_kernel();
}

/// set trap entry in supervisor mode
pub fn set_kernel_trap_entry() {
    set_trap_entry(trap_from_kernel as usize);
}

/// set trap entry in user mode
pub fn set_user_trap_entry() {
    set_trap_entry(user_trapvec as usize);
}

/// trap init of current hart
pub fn trap_init() {
    set_kernel_trap_entry();
    enable_external_interrupt();
    enable_global_interrupt();
    enable_software_interrupt();
    enable_stimer_interrupt();
}

#[no_mangle]
/// kernel back to user
pub fn trap_restore(cx: &mut TrapContext) {
    set_user_trap_entry();
    unsafe { user_trapret(cx) };
}
