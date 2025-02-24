use alloc::sync::Arc;

use crate::{include::signal::sig_set::SigMask, syscall::SyscallResult, task::Task};

extern "C" {
    fn user_sigreturn();
}

impl Task {
    pub fn check_signal(self: &Arc<Self>) -> SyscallResult {
        Ok(0)
    }
    pub fn set_wake_signal(self: &Arc<Self>, should_wake: SigMask) {
        self.pending_sigs().should_wake = should_wake;
    }
}
