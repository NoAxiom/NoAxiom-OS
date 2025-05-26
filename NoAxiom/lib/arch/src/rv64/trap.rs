use core::{
    arch::{asm, global_asm},
    intrinsics::volatile_load,
};

use config::cpu::CPU_NUM;
use riscv::register::{
    scause::{self, Exception, Interrupt, Scause, Trap},
    sepc,
    sstatus::FS,
    stval,
    stvec::{self, TrapMode},
};

use super::{context::TrapContext, interrupt::{disable_interrupt, enable_interrupt}, RV64};
use crate::{
    rv64::interrupt::{
        enable_external_interrupt, enable_software_interrupt, enable_stimer_interrupt,
        enable_user_memory_access,
    },
    ArchAsm, ArchInt, ArchTrap, ArchTrapContext, ArchUserFloatContext, TrapType, UserPtrResult,
};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn __user_trapvec();
    fn __user_trapret(cx: *mut TrapContext);
    fn __kernel_trapvec();
    fn __kernel_user_ptr_vec();
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
fn set_ptr_entry() {
    set_trap_entry(__kernel_user_ptr_vec as usize);
}

#[repr(align(64))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Wrapper(TrapType);
static mut USER_PTR_TRAP_TYPE: [Wrapper; CPU_NUM] = [Wrapper(TrapType::None); CPU_NUM];

unsafe fn before_user_ptr() {
    RV64::disable_interrupt();
    set_ptr_entry();
    USER_PTR_TRAP_TYPE[RV64::get_hartid()] = Wrapper(TrapType::None);
}

unsafe fn after_user_ptr() -> UserPtrResult {
    let trap_type = volatile_load(&USER_PTR_TRAP_TYPE[RV64::get_hartid()]).0;
    let res = match trap_type {
        TrapType::None => Ok(()),
        _ => Err(trap_type),
    };
    set_kernel_trap_entry();
    RV64::enable_interrupt();
    res
}

unsafe fn bare_read(ptr: usize) {
    asm!(
        ".option push
        .option norvc
        lb a0, 0(a0)
        .option pop",
        in("a0") ptr,
    );
}

unsafe fn check_read(ptr: usize) -> UserPtrResult {
    before_user_ptr();
    bare_read(ptr);
    after_user_ptr()
}

unsafe fn bare_write(ptr: usize) {
    asm!(
        ".option push
        .option norvc
        lb t0, 0(a0)
        sb t0, 0(a0)
        .option pop",
        in("a0") ptr,
    );
}

unsafe fn check_write(ptr: usize) -> UserPtrResult {
    before_user_ptr();
    bare_write(ptr);
    after_user_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn kernel_user_ptr_handler() {
    let hartid = RV64::get_hartid();
    let scause = scause::read();
    let stval = stval::read();
    let sepc = sepc::read();
    sepc::write(sepc + 4); // skip read
    USER_PTR_TRAP_TYPE[hartid] = Wrapper(get_trap_type(scause, stval));
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
        enable_interrupt();
        disable_interrupt();
        set_user_trap_entry();
        cx.freg_mut().restore();
        cx.sstatus().set_fs(FS::Clean);
        unsafe { __user_trapret(cx) };
        set_kernel_trap_entry();
        let fs = cx.sstatus().fs();
        cx.freg_mut().mark_save_if_needed(fs);
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
    /// try read user ptr
    fn check_read(addr: usize) -> UserPtrResult {
        unsafe { check_read(addr) }
    }
    /// try write user ptr
    fn check_write(addr: usize) -> UserPtrResult {
        unsafe { check_write(addr) }
    }
}
