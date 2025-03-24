use alloc::collections::vec_deque::VecDeque;

use super::{sig_info::SigInfo, sig_set::SigSet};

/// pending signals of a task
/// it stores signals that are pending to be handled.
/// note that: even if the signal is masked,
/// it will still be stored in the pending signals,
/// and will be handled when the signal is unmasked
pub struct SigPending {
    pub queue: VecDeque<SigInfo>, // pending signal queue that should be handled
    pub pending_set: SigSet,      // current pending signal set, used to avoid duplicate signals
    pub should_wake: SigSet,      // signals that should wake the task
}

impl SigPending {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            pending_set: SigSet::empty(),
            should_wake: SigSet::empty(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn push(&mut self, sig_info: SigInfo) {
        if !self.pending_set.contain_signum(sig_info.signo as u32) {
            self.pending_set.enable(sig_info.signo as u32);
            self.queue.push_back(sig_info);
        }
    }

    pub fn pop_one(&mut self) -> Option<SigInfo> {
        if let Some(sig_info) = self.queue.pop_front() {
            self.pending_set.disable(sig_info.signo as u32);
            Some(sig_info)
        } else {
            None
        }
    }

    pub fn pop_with_mask(&mut self, mask: SigSet) -> Option<SigInfo> {
        let x = self.pending_set & mask;
        if x.is_empty() {
            return None;
        } else {
            for i in 0..self.queue.len() {
                let signo = self.queue[i].signo;
                if x.contain_signum(signo as u32) {
                    self.pending_set.disable(signo as u32);
                    return self.queue.remove(i);
                }
            }
            error!("[pop_with_mask] signal not found");
            return None;
        }
    }
}
