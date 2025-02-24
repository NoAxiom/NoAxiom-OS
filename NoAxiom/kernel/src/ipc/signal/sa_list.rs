use crate::{constant::signal::MAX_SIGNUM, include::signal::sig_action::KernelSigAction};

/// signal action list of a task
pub struct SigActionList {
    pub actions: [KernelSigAction; MAX_SIGNUM as usize],
}

impl SigActionList {
    pub fn new() -> Self {
        Self {
            actions: [KernelSigAction::default(); MAX_SIGNUM as usize],
        }
    }
}

// todo: should complete sigaction
