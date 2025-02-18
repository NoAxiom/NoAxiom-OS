use alloc::collections::vec_deque::VecDeque;
use core::task::Waker;

use super::{sig_info::SigInfo, sig_set::SigMask};

pub struct PendingSignals {
    pub signal: SigMask,
    pub sigs_queue: VecDeque<SigInfo>,
    pub waker: Option<Waker>,
    pub should_wake: SigMask,
}
