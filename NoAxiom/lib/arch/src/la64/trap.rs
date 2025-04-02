use core::arch::global_asm;

use log::error;
use loongArch64::register::{
    badv, ecfg, eentry, era,
    estat::{self, Exception, Interrupt, Trap},
};

use super::{
    context::TrapContext,
    interrupt::{disable_interrupt, interrupt_init},
    unaligned::emulate_load_store_insn,
    LA64,
};
use crate::{ArchTrap, TrapType};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn user_trapvec();
    fn user_trapret(cx: *mut TrapContext);
    fn kernel_trapvec();
}

#[inline]
pub fn set_user_trap_entry() {
    ecfg::set_vs(0);
    eentry::set_eentry(user_trapvec as usize);
}

#[inline]
pub fn set_kernel_trap_entry() {
    ecfg::set_vs(0);
    eentry::set_eentry(kernel_trapvec as usize);
}

fn get_trap_type(tf: Option<&mut TrapContext>) -> TrapType {
    let estat = estat::read();
    let badv = badv::read().vaddr();
    match estat.cause() {
        Trap::Exception(e) => match e {
            Exception::Breakpoint => TrapType::Breakpoint,
            Exception::AddressNotAligned => {
                unsafe { emulate_load_store_insn(tf.unwrap()) }
                TrapType::None
            }
            Exception::Syscall => TrapType::SysCall,
            Exception::StorePageFault | Exception::PageModifyFault => {
                TrapType::StorePageFault(badv)
            }
            Exception::PageNonExecutableFault
            | Exception::FetchPageFault
            | Exception::FetchInstructionAddressError
            | Exception::InstructionPrivilegeIllegal => TrapType::InstructionPageFault(badv),
            Exception::LoadPageFault
            | Exception::PageNonReadableFault
            | Exception::MemoryAccessAddressError
            | Exception::PagePrivilegeIllegal => TrapType::LoadPageFault(badv),
            _ => {
                error!(
                    "[get_trap_type] unhandled exception: {:?}, pc = {:#x}, BADV = {:#x}",
                    e,
                    era::read().pc(),
                    badv,
                );
                error!("[get_trap_type] trap_cx: {:#x?}", tf);
                TrapType::Unknown
            }
        },
        Trap::Interrupt(int) => match int {
            Interrupt::Timer => TrapType::Timer,
            Interrupt::HWI0
            | Interrupt::HWI1
            | Interrupt::HWI2
            | Interrupt::HWI3
            | Interrupt::HWI4
            | Interrupt::HWI5
            | Interrupt::HWI6
            | Interrupt::HWI7 => TrapType::SupervisorExternal,
            Interrupt::SWI0 | Interrupt::SWI1 | Interrupt::IPI => TrapType::SupervisorSoft,
            _ => {
                error!(
                    "[get_trap_type] unhandled interrupt: {:?}, pc = {:#x}, BADV = {:#x}",
                    int,
                    era::read().pc(),
                    badv,
                );
                error!("[get_trap_type] trap_cx: {:#x?}", tf);
                TrapType::Unknown
            }
        },
        _ => {
            error!(
                "[get_trap_type] unhandled trap type: {:?}, pc = {:#x}, BADV = {:#x}, raw_ecode = {}, esubcode = {}, is = {}",
                estat.cause(),
                era::read().pc(),
                badv,
                estat.ecode(),
                estat.esubcode(),
                estat.is()
            );
            error!("[get_trap_type] trap_cx: {:#x?}", tf);
            TrapType::Unknown
        }
    }
}

pub(crate) fn trap_init() {
    set_kernel_trap_entry();
    interrupt_init();
}

impl ArchTrap for LA64 {
    type TrapContext = TrapContext;
    fn read_trap_type(cx: Option<&mut TrapContext>) -> crate::TrapType {
        get_trap_type(cx)
    }
    fn set_kernel_trap_entry() {
        set_kernel_trap_entry();
    }
    fn trap_init() {
        trap_init();
    }
    fn read_epc() -> usize {
        era::read().pc()
    }
    fn trap_restore(cx: &mut TrapContext) {
        debug!("[trap_restore] era: {:#x}, sp: {:#x}", cx.era, cx.x[3]);
        disable_interrupt();
        set_user_trap_entry();
        unsafe { user_trapret(cx) };
    }
    fn set_user_trap_entry() {
        set_user_trap_entry();
    }
}
