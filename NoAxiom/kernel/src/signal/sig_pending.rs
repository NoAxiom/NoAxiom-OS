use alloc::collections::vec_deque::VecDeque;

use super::{
    sig_info::SigInfo,
    sig_set::{SigMask, SigSet},
};

/// pending signals of a task
/// it stores signals that are pending to be handled.
/// note that: even if the signal is masked,
/// it will still be stored in the pending signals,
/// and will be handled when the signal is unmasked
pub struct SigPending {
    pub sig_mask: SigMask,        // signal mask of the task
    pub queue: VecDeque<SigInfo>, // pending signal queue that should be handled
    pub pending_set: SigSet,      // current pending signal set, used to avoid duplicate signals
    pub should_wake: SigSet,      // signals that should wake the task
}

impl SigPending {
    pub fn new() -> Self {
        Self {
            sig_mask: SigMask::empty(),
            queue: VecDeque::new(),
            pending_set: SigSet::empty(),
            should_wake: SigSet::all(),
        }
    }

    pub fn push(&mut self, sig_info: SigInfo) {
        if !self.pending_set.contains_signal(sig_info.signal) {
            self.pending_set.enable(sig_info.signal);
            self.queue.push_back(sig_info);
        }
    }

    pub fn pop_with_mask(&mut self, mask: SigMask) -> Option<SigInfo> {
        // kill / stop signal cannot be blocked
        // other signals can be blocked by sigmask
        let accept_set = self.pending_set & !mask.without_kill();
        if accept_set.is_empty() {
            return None;
        } else {
            for i in 0..self.queue.len() {
                if accept_set.contains_signal(self.queue[i].signal) {
                    let _ = self
                        .pending_set
                        .disable(self.queue[i].signal)
                        .inspect_err(|err| {
                            error!(
                                "disable signal failed, signal: {:?}, err: {:?}",
                                self.queue[i].signal, err
                            )
                        });
                    return self.queue.remove(i);
                }
            }
            error!("[pop_with_mask] signal not found");
            return None;
        }
    }

    pub fn peek_with_mask(&self, mask: &SigMask) -> Option<&SigInfo> {
        let accept_set = self.pending_set & !mask.without_kill();
        if accept_set.is_empty() {
            return None;
        } else {
            for i in 0..self.queue.len() {
                if accept_set.contains_signal(self.queue[i].signal) {
                    return Some(&self.queue[i]);
                }
            }
            error!("[pop_with_mask] signal not found");
            return None;
        }
    }

    pub fn has_expect_signals(&self, expect: SigSet) -> bool {
        !((expect & self.pending_set).is_empty())
    }

    pub fn get_pending_signals_with_expect(&self, expect: SigSet) -> SigSet {
        expect & self.pending_set
    }
}

impl Default for SigPending {
    fn default() -> Self {
        Self::new()
    }
}
