use alloc::sync::Arc;

use crate::{
    include::signal::{sig_info::SigInfo, sig_set::SigMask},
    syscall::SyscallResult,
    task::Task,
};

extern "C" {
    fn user_sigreturn();
}

impl Task {
    pub fn check_signal(self: &Arc<Self>) -> SyscallResult {
        // todo: check_signal
        Ok(0)
    }
    pub fn set_wake_signal(self: &Arc<Self>, should_wake: SigMask) {
        self.pending_sigs().should_wake = should_wake;
    }
    pub fn proc_recv_siginfo(self: &Arc<Self>, siginfo: SigInfo) {
        // todo: complete proc_recv_siginfo
        self.pending_sigs().push(siginfo);
        self.waker().as_ref().unwrap().wake_by_ref();
    }
}
