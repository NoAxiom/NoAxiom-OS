use core::arch::global_asm;

use riscv::register::{
    scause::{self, Exception, Interrupt, Scause, Trap},
    sepc,
    sstatus::FS,
    stval,
    stvec::{self, TrapMode},
};

use super::{context::TrapContext, interrupt::disable_interrupt, RV64};
use crate::{
    rv64::interrupt::{
        enable_external_interrupt, enable_software_interrupt, enable_stimer_interrupt,
        enable_user_memory_access,
    },
    ArchTrap, ArchTrapContext, ArchUserFloatContext, TrapType,
};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn __user_trapvec();
    fn __user_trapret(cx: *mut TrapContext);
    fn __kernel_trapvec();
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
        _ => panic!("unknown trap type: {:?}", scause.cause()),
    }
}

#[inline(always)]
pub fn set_trap_entry(addr: usize) {
    unsafe { stvec::write(addr, TrapMode::Direct) };
}

fn set_kernel_trap_entry() {
    set_trap_entry(__kernel_trapvec as usize);
}
fn set_user_trap_entry() {
    set_trap_entry(__user_trapvec as usize);
}

pub fn trap_init() {
    set_kernel_trap_entry();
    enable_user_memory_access();
    enable_external_interrupt();
    enable_software_interrupt();
    enable_stimer_interrupt();
}

impl ArchTrap for RV64 {
    type TrapContext = super::context::TrapContext;
    /// init trap in a single hart
    /// note that it won't turn on global interrupt
    fn trap_init() {
        trap_init();
    }
    /// restore trap context, with freg handled as well
    fn trap_restore(cx: &mut TrapContext) {
        disable_interrupt();
        set_user_trap_entry();
        cx.freg_mut().restore();
        cx.sstatus().set_fs(FS::Clean);
        unsafe { __user_trapret(cx) };
        set_kernel_trap_entry();
        cx.freg_mut().mark_save_if_needed();
    }
    /// read exception pc
    fn read_epc() -> usize {
        sepc::read()
    }
    /// translate scause and stval to common TrapType
    fn read_trap_type(_: Option<&mut TrapContext>) -> TrapType {
        let scause = scause::read();
        let stval = stval::read();
        get_trap_type(scause, stval)
    }
}
