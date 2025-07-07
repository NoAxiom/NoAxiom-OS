use core::task::Waker;

use arch::TrapContext;

use super::context::TaskTrapContext;
use crate::{
    include::{process::TaskFlags, syscall_id::SyscallID},
    mm::user_ptr::UserPtr,
    signal::{sig_set::SigMask, sig_stack::UContext},
};

pub struct TCB {
    pub flags: TaskFlags,               // thread flags
    pub sig_mask: SigMask,              // signal mask of the task
    pub old_mask: Option<SigMask>,              // old signal mask
    pub waker: Option<Waker>,           // waker for the task
    pub cx: TaskTrapContext,            // trap context
    pub ucx: UserPtr<UContext>,         // ucontext for the task
    pub set_child_tid: Option<usize>,   // set tid address
    pub clear_child_tid: Option<usize>, // clear tid address
    pub current_syscall: SyscallID,     // current syscall id
}

impl Default for TCB {
    fn default() -> Self {
        Self {
            flags: TaskFlags::empty(),
            sig_mask: SigMask::empty(),
            old_mask: None,
            waker: None,
            cx: TaskTrapContext::new(TrapContext::default(), true),
            ucx: UserPtr::new_null(),
            set_child_tid: None,
            clear_child_tid: None,
            current_syscall: SyscallID::NO_SYSCALL,
        }
    }
}
