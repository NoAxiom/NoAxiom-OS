use alloc::collections::vec_deque::VecDeque;
use core::task::Waker;

use crate::include::signal::{sig_info::SigInfo, sig_set::SigSet};

/// pending signals of a task
pub struct PendingSigs {
    pub sigset: SigSet,           // pending signal set
    pub queue: VecDeque<SigInfo>, // pending signal queue
    pub should_wake: SigSet,      // signals that should wake the task
}

impl PendingSigs {
    pub fn new() -> Self {
        Self {
            sigset: SigSet::empty(),
            queue: VecDeque::new(),
            should_wake: SigSet::empty(),
        }
    }

    pub fn push(&mut self, sig_info: SigInfo, waker: Option<Waker>) {
        if self.sigset.has_signum(sig_info.signo as u32) {
            return;
        } else {
            self.sigset.enable(sig_info.signo as u32);
            self.queue.push_back(sig_info);
        }
        if let Some(waker) = waker.as_ref() {
            waker.wake_by_ref();
        }
    }

    pub fn pop_one(&mut self) -> Option<SigInfo> {
        if let Some(sig_info) = self.queue.pop_front() {
            self.sigset.disable(sig_info.signo as u32);
            Some(sig_info)
        } else {
            None
        }
    }

    pub fn pop_with_mask(&mut self, mask: SigSet) -> Option<SigInfo> {
        let x = self.sigset & mask;
        if x.is_empty() {
            return None;
        } else {
            for i in 0..self.queue.len() {
                let signo = self.queue[i].signo;
                if x.has_signum(signo as u32) {
                    self.sigset.disable(signo as u32);
                    return self.queue.remove(i);
                }
            }
            error!("[pop_with_mask] signal not found");
            return None;
        }
    }
}
