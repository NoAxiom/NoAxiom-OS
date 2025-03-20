use array_init::array_init;

use super::sig_action::KSigAction;
use crate::constant::signal::MAX_SIGNUM;

/// signal action list of a task
pub struct SigActionList {
    pub actions: [KSigAction; MAX_SIGNUM as usize],
}

impl SigActionList {
    pub fn new() -> Self {
        Self {
            actions: array_init(|signo| KSigAction::new_default(signo.into())),
        }
    }
    pub fn set_sigaction(&mut self, signum: usize, action: KSigAction) {
        self.actions[signum] = action;
    }
    pub fn get(&self, signum: usize) -> Option<&KSigAction> {
        self.actions.get(signum)
    }
}

// todo: should complete sigaction
