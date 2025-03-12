use core::arch::global_asm;

use riscv::register::{
    scause, sepc, stval,
    stvec::{self, TrapMode},
};

use super::{context::TrapContext, RV64};
use crate::{
    rv64::interrupt::{
        enable_external_interrupt, enable_global_interrupt, enable_software_interrupt,
        enable_stimer_interrupt,
    },
    ArchTrap,
};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn user_trapvec();
    fn user_trapret(cx: *mut TrapContext);
    fn trap_from_kernel();
}

#[inline(always)]
pub fn set_trap_entry(addr: usize) {
    unsafe { stvec::write(addr, TrapMode::Direct) };
}

/// trap init of current hart
pub fn trap_init() {
    RV64::set_kernel_trap_entry();
    enable_external_interrupt();
    enable_global_interrupt();
    enable_software_interrupt();
    enable_stimer_interrupt();
}

#[no_mangle]
/// kernel back to user
pub fn trap_restore(cx: &mut TrapContext) {
    RV64::set_user_trap_entry();
    unsafe { user_trapret(cx) };
}

impl ArchTrap for RV64 {
    #[inline(always)]
    fn set_trap_entry(addr: usize) {
        set_trap_entry(addr);
    }
    fn read_trap_cause() -> Self::Trap {
        scause::read().cause()
    }
    fn read_trap_value() -> usize {
        stval::read()
    }
    fn read_trap_pc() -> usize {
        sepc::read()
    }
    /// set trap entry in supervisor mode
    fn set_kernel_trap_entry() {
        set_trap_entry(trap_from_kernel as usize);
    }
    /// set trap entry in user mode
    fn set_user_trap_entry() {
        set_trap_entry(user_trapvec as usize);
    }
    fn trap_init() {
        trap_init();
    }
    fn trap_restore(cx: &mut TrapContext) {
        trap_restore(cx);
    }
}
