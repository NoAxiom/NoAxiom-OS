use alloc::sync::Arc;

use ksync::{cell::SyncUnsafeCell, mutex::SpinLock};

use super::{sig_action::SigActionList, sig_pending::SigPending, sig_set::SigMask};

pub struct SignalControlBlock {
    /// pending signals, saves signals not handled
    pub pending_sigs: SpinLock<SigPending>,

    /// signal action list, saves signal handler
    pub sa_list: Arc<SpinLock<SigActionList>>,

    /// signal mask, for those signals should be blocked
    pub sig_mask: SyncUnsafeCell<SigMask>,
    //
    // /// signal ucontext
    // sig_ucontext_cx: AtomicUsize,
    //
    // /// signal stack
    // pub sigstack: Option<SignalStack>,
}

impl SignalControlBlock {
    pub fn new(sa_list: Option<&Arc<SpinLock<SigActionList>>>) -> Self {
        Self {
            pending_sigs: SpinLock::new(SigPending::new()),
            sa_list: sa_list
                .map(|p| p.clone())
                .unwrap_or_else(|| Arc::new(SpinLock::new(SigActionList::new()))),
            sig_mask: SyncUnsafeCell::new(SigMask::empty()),
            // sig_ucontext_cx: SyncUnsafeCell::new(SigContext::empty()),
        }
    }
}
