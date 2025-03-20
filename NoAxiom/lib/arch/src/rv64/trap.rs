use core::arch::global_asm;

use riscv::register::{
    scause::{self, Exception, Interrupt, Scause, Trap},
    sepc, stval,
    stvec::{self, TrapMode},
};

use super::{context::TrapContext, RV64};
use crate::{
    rv64::interrupt::{
        enable_external_interrupt, enable_global_interrupt, enable_software_interrupt,
        enable_stimer_interrupt,
    },
    ArchTrap, TrapType,
};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn user_trapvec();
    fn user_trapret(cx: *mut TrapContext);
    fn kernel_trapvec();
}

pub fn get_trap_type(scause: Scause, stval: usize) -> TrapType {
    match scause.cause() {
        Trap::Exception(Exception::LoadFault) => TrapType::Unknown,
        Trap::Exception(Exception::UserEnvCall) => TrapType::SysCall,
        Trap::Interrupt(Interrupt::SupervisorTimer) => TrapType::Timer,
        Trap::Exception(Exception::StorePageFault) => TrapType::StorePageFault(stval),
        Trap::Exception(Exception::StoreFault) => TrapType::StorePageFault(stval),
        Trap::Exception(Exception::InstructionPageFault) => TrapType::InstructionPageFault(stval),
        Trap::Exception(Exception::IllegalInstruction) => TrapType::IllegalInstruction(stval),
        Trap::Exception(Exception::LoadPageFault) => TrapType::LoadPageFault(stval),
        Trap::Interrupt(Interrupt::SupervisorExternal) => TrapType::SupervisorExternal,
        Trap::Interrupt(Interrupt::SupervisorSoft) => TrapType::SupervisorSoft,
        _ => panic!("unknown trap type"),
    }
}

#[inline(always)]
pub fn set_trap_entry(addr: usize) {
    unsafe { stvec::write(addr, TrapMode::Direct) };
}

/// trap init of current hart
pub fn trap_init() {
    RV64::set_kernel_trap_entry();
    enable_external_interrupt();
    enable_software_interrupt();
    enable_stimer_interrupt();
    enable_global_interrupt();
}

#[no_mangle]
/// kernel back to user
pub fn trap_restore(cx: &mut TrapContext) {
    RV64::set_user_trap_entry();
    unsafe { user_trapret(cx) };
}

impl ArchTrap for RV64 {
    type TrapContext = super::context::TrapContext;
    /// set trap entry in supervisor mode
    fn set_kernel_trap_entry() {
        set_trap_entry(kernel_trapvec as usize);
    }
    /// set trap entry in user mode
    fn set_user_trap_entry() {
        set_trap_entry(user_trapvec as usize);
    }
    fn trap_init() {
        trap_init();
    }
    fn trap_restore(cx: &mut TrapContext) {
        trap_restore(cx)
    }
    fn read_epc() -> usize {
        sepc::read()
    }
    fn read_trap_type() -> TrapType {
        let scause = scause::read();
        let stval = stval::read();
        get_trap_type(scause, stval)
    }
}
