use alloc::sync::Arc;

use ksync::{cell::SyncUnsafeCell, mutex::SpinLock};

use super::{pending_sigs::PendingSigs, sa_list::SigActionList};
use crate::include::signal::sig_set::SigMask;

pub struct SignalInfo {
    /// pending signals
    pub pending_sigs: Arc<SpinLock<PendingSigs>>,

    /// signal action list
    pub sa_list: Arc<SpinLock<SigActionList>>,

    /// signal mask
    pub sig_mask: SyncUnsafeCell<SigMask>,
    //
    // /// signal ucontext
    // sig_ucontext_cx: AtomicUsize,
    //
    // /// signal stack
    // pub sigstack: Option<SignalStack>,
}

impl SignalInfo {
    pub fn new(
        pending_sigs: Option<&Arc<SpinLock<PendingSigs>>>,
        sa_list: Option<&Arc<SpinLock<SigActionList>>>,
    ) -> Self {
        Self {
            pending_sigs: pending_sigs
                .map(|p| p.clone())
                .unwrap_or_else(|| Arc::new(SpinLock::new(PendingSigs::new()))),
            sa_list: sa_list
                .map(|p| p.clone())
                .unwrap_or_else(|| Arc::new(SpinLock::new(SigActionList::new()))),
            sig_mask: SyncUnsafeCell::new(SigMask::empty()),
            // sig_ucontext_cx: SyncUnsafeCell::new(SigContext::empty()),
        }
    }
}
