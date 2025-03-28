use core::arch::global_asm;

use loongArch64::register::{ecfg, eentry, era};

use super::{context::TrapContext, LA64};
use crate::ArchTrap;

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

impl ArchTrap for LA64 {
    type TrapContext = TrapContext;
    fn read_trap_type() -> crate::TrapType {
        unimplemented!();
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
    fn trap_restore(_cx: &mut <Self as ArchTrap>::TrapContext) {
        set_user_trap_entry();
        unimplemented!()
    }
    fn set_user_trap_entry() {
        set_user_trap_entry();
        unimplemented!()
    }
}
