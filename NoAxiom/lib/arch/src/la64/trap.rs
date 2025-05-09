use core::arch::global_asm;

use log::error;
use loongArch64::register::{
    badi, badv, ecfg, eentry, era,
    estat::{self, Exception, Interrupt, Trap},
};

use super::{
    context::TrapContext,
    interrupt::{disable_interrupt, interrupt_init},
    unaligned::emulate_load_store_insn,
    LA64,
};
use crate::{ArchTrap, ArchTrapContext, ArchUserFloatContext, TrapType};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn __user_trapvec();
    fn __user_trapret(cx: *mut TrapContext);
    fn __kernel_trapvec();
}

#[inline]
pub fn set_user_trap_entry() {
    ecfg::set_vs(0);
    eentry::set_eentry(__user_trapvec as usize);
}

#[inline]
pub fn set_kernel_trap_entry() {
    ecfg::set_vs(0);
    eentry::set_eentry(__kernel_trapvec as usize);
}

fn get_trap_type(tf: Option<&mut TrapContext>) -> TrapType {
    let estat = estat::read();
    let badv = badv::read().vaddr();
    match estat.cause() {
        Trap::Exception(Exception::Syscall) | Trap::Interrupt(_) => {}
        _ => {
            info!(
                "[get_trap_type] estat: {:x?}, badv: {:#x}, pc: {:#x}",
                estat.cause(),
                badv,
                era::read().pc(),
            );
        }
    }
    match estat.cause() {
        Trap::Exception(e) => {
            match e {
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
                Exception::InstructionNotExist => {
                    error!(
                        "[get_trap_type] InstructionNotExist, pc = {:#x}, BADV = {:#x}, BADI = {:#x}",
                        era::read().pc(), badv, badi::read().inst()
                    );
                    TrapType::IllegalInstruction(badv)
                }
                _ => {
                    error!(
                    "[get_trap_type] unhandled exception: {:?}, pc = {:#x}, BADV = {:#x}, BADI = {:#x}",
                    e,
                    era::read().pc(),
                    badv,
                    badi::read().inst(),
                );
                    // error!("[get_trap_type] trap_cx: {:#x?}", tf);
                    TrapType::Unknown
                }
            }
        }
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
                // error!("[get_trap_type] trap_cx: {:#x?}", tf);
                TrapType::Unknown
            }
        },
        _ => {
            error!(
                "[get_trap_type] unhandled trap type: {:?}, pc = {:#x}, BADV = {:#x}, raw_ecode = {:#x}, esubcode = {:#x}, badi: {:#x}, is = {}",
                estat.cause(),
                era::read().pc(),
                badv,
                estat.ecode(),
                estat.esubcode(),
                badi::read().inst(),
                estat.is()
            );
            // error!("[get_trap_type] trap_cx: {:#x?}", tf);
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
        // debug!("[trap_restore] era: {:#x}, sp: {:#x}", cx.era, cx.x[3]);
        disable_interrupt();
        set_user_trap_entry();
        cx.freg_mut().restore();
        unsafe { __user_trapret(cx) };
        cx.freg_mut().mark_save_if_needed();
        disable_interrupt();
        set_kernel_trap_entry();
    }
    fn set_user_trap_entry() {
        set_user_trap_entry();
    }
}
