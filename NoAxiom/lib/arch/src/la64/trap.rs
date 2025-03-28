use core::arch::global_asm;

use super::{context::TrapContext, LA64};
use crate::ArchTrap;

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn user_trapvec();
    fn user_trapret(cx: *mut TrapContext);
    fn kernel_trapvec();
}

impl ArchTrap for LA64 {
    type TrapContext = TrapContext;
    fn read_trap_type() -> crate::TrapType {
        unimplemented!();
    }
    fn set_kernel_trap_entry() {
        unimplemented!();
    }
    fn trap_init() {
        unimplemented!()
    }
    fn read_epc() -> usize {
        unimplemented!()
    }
    fn trap_restore(_cx: &mut <Self as ArchTrap>::TrapContext) {
        unimplemented!()
    }
    fn set_user_trap_entry() {
        unimplemented!()
    }
}
