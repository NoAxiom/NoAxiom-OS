use core::task::Waker;

use arch::TrapContext;

use super::context::TaskTrapContext;
use crate::{
    include::{process::ThreadInfo, syscall_id::SyscallID},
    mm::user_ptr::UserPtr,
    signal::sig_stack::UContext,
};

pub struct TCB {
    pub tif: ThreadInfo,                // thread flags
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
            tif: ThreadInfo::empty(),
            waker: None,
            cx: TaskTrapContext::new(TrapContext::default(), true),
            ucx: UserPtr::new_null(),
            set_child_tid: None,
            clear_child_tid: None,
            current_syscall: SyscallID::NO_SYSCALL,
        }
    }
}
