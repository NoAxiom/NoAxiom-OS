use config::cpu::CPU_NUM;
use ksync::cell::SyncUnsafeCell;
use lazy_static::lazy_static;

use crate::{cpu::get_hartid, include::syscall_id::SyscallID};

#[cfg(feature = "debug_sig")]
lazy_static! {
    pub static ref CURRENT_SYSCALL: [SyncUnsafeCell<SyscallID>; CPU_NUM] =
        array_init::array_init(|_| SyncUnsafeCell::new(SyscallID::NO_SYSCALL));
}

pub fn current_syscall() -> SyscallID {
    *CURRENT_SYSCALL[get_hartid()].as_ref()
}

pub fn update_current_syscall(syscall_id: SyscallID) {
    *CURRENT_SYSCALL[get_hartid()].as_ref_mut() = syscall_id;
}

pub fn clear_current_syscall() {
    *CURRENT_SYSCALL[get_hartid()].as_ref_mut() = SyscallID::NO_SYSCALL;
}