use core::arch::global_asm;

use loongArch64::register::{
    badv, ecfg, eentry, era,
    estat::{self, Exception, Trap},
};

use super::{context::TrapContext, unaligned::emulate_load_store_insn, LA64};
use crate::{ArchInt, ArchTrap, TrapType};

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
                TrapType::Unknown
            }
            Exception::Syscall => TrapType::SysCall,
            Exception::StorePageFault | Exception::PageModifyFault => {
                TrapType::StorePageFault(badv)
            }
            Exception::PageNonExecutableFault | Exception::FetchPageFault => {
                TrapType::InstructionPageFault(badv)
            }
            Exception::LoadPageFault | Exception::PageNonReadableFault => {
                TrapType::LoadPageFault(badv)
            }
            _ => panic!(
                "Unhandled trap {:?} @ {:#x} BADV: {:#x}:\n{:#x?}",
                estat.cause(),
                era::read().pc(),
                badv,
                tf
            ),
        },
        Trap::Interrupt(_) => {
            let irq_num: usize = estat.is().trailing_zeros() as usize;
            match irq_num {
                // TIMER_IRQ
                7 => TrapType::Timer,
                _ => panic!("unknown interrupt: {}", irq_num),
            }
        }
        _ => panic!(
            "Unhandled trap {:?} @ {:#x} BADV: {:#x}:\n{:#x?}",
            estat.cause(),
            era::read().pc(),
            badv,
            tf
        ),
    }
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
        set_kernel_trap_entry();
    }
    fn read_epc() -> usize {
        era::read().pc()
    }
    fn trap_restore(cx: &mut TrapContext) {
        LA64::disable_global_interrupt();
        set_user_trap_entry();
        unsafe { user_trapret(cx) };
    }
    fn set_user_trap_entry() {
        set_user_trap_entry();
    }
}
