use core::{
    arch::{asm, global_asm},
    intrinsics::volatile_load,
};

use config::cpu::CPU_NUM;
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
use crate::{
    la64::interrupt::is_interrupt_enabled, ArchAsm, ArchInt, ArchTrap, ArchTrapContext,
    ArchUserFloatContext, TrapType, UserPtrResult,
};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn __user_trapvec();
    fn __user_trapret(cx: *mut TrapContext);
    fn __kernel_trapvec();
    fn __kernel_user_ptr_vec();
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

#[inline]
fn set_ptr_entry() {
    ecfg::set_vs(0);
    eentry::set_eentry(__kernel_user_ptr_vec as usize);
}

#[repr(align(64))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Wrapper(TrapType);
static mut USER_PTR_TRAP_TYPE: [Wrapper; CPU_NUM] = [Wrapper(TrapType::None); CPU_NUM];

unsafe fn before_user_ptr() {
    LA64::disable_interrupt();
    set_ptr_entry();
    USER_PTR_TRAP_TYPE[LA64::get_hartid()] = Wrapper(TrapType::None);
}

unsafe fn after_user_ptr() -> UserPtrResult {
    let trap_type = volatile_load(&USER_PTR_TRAP_TYPE[LA64::get_hartid()]).0;
    let res = match trap_type {
        TrapType::None => Ok(()),
        _ => Err(trap_type),
    };
    set_kernel_trap_entry();
    LA64::enable_interrupt();
    res
}

unsafe fn bare_read(ptr: usize) {
    asm!("ld.b $a0, $a0, 0", in("$a0") ptr);
}

unsafe fn check_read(ptr: usize) -> UserPtrResult {
    before_user_ptr();
    bare_read(ptr);
    after_user_ptr()
}

unsafe fn bare_write(ptr: usize) {
    asm!("ld.b $t0, $a0, 0; st.b $t0, $a0, 0", in("$a0") ptr);
}

unsafe fn check_write(ptr: usize) -> UserPtrResult {
    before_user_ptr();
    bare_write(ptr);
    after_user_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn kernel_user_ptr_handler() {
    let hartid = LA64::get_hartid();
    let pc = era::read().pc();
    era::set_pc(pc + 4);
    USER_PTR_TRAP_TYPE[hartid] = Wrapper(get_trap_type(None));
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
        set_kernel_trap_entry();
        cx.freg_mut().mark_save_if_needed();
        assert!(!is_interrupt_enabled());
    }
    /// try read user ptr
    fn check_read(addr: usize) -> UserPtrResult {
        unsafe { check_read(addr) }
    }
    /// try write user ptr
    fn check_write(addr: usize) -> UserPtrResult {
        unsafe { check_write(addr) }
    }
}
