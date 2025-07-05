use array_init::array_init;
use bitflags::bitflags;

use super::{
    sig_set::{SigMask, SigSet},
    signal::Signal,
};
use crate::signal::signal::{MAX_SIGNUM, SIG_DFL, SIG_IGN};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SAHandlerType {
    Ignore,
    Kill,
    Stop,
    Continue,
    User { handler: usize }, // handler addr
}

impl SAHandlerType {
    pub const fn new_default(sig: Signal) -> Self {
        match sig {
            Signal::SIGCHLD | Signal::SIGURG | Signal::SIGWINCH => Self::Ignore,
            Signal::SIGSTOP | Signal::SIGTSTP | Signal::SIGTTIN | Signal::SIGTTOU => Self::Stop,
            Signal::SIGCONT => Self::Continue,
            _ => Self::Kill,
        }
    }
}

bitflags! {
    #[repr(C)]
    #[derive(Copy, Clone, Debug)]
    pub struct SAFlags: u32 {
        const SA_NOCLDSTOP = 1; /* Don't send SIGCHLD when children stop.  */
        const SA_NOCLDWAIT = 2; /* Don't create zombie on child death.  */
        const SA_SIGINFO   = 4; /* Invoke signal-catching function with
                                   three arguments instead of one.  */
        const SA_ONSTACK   = 0x08000000; /* Use signal stack by using `sa_restorer'. */
        const SA_RESTART   = 0x10000000; /* Restart syscall on signal return.  */
        const SA_NODEFER   = 0x40000000; /* Don't automatically block the signal when
                                            its handler is being executed.  */
        const SA_RESETHAND = 0x80000000; /* Reset to SIG_DFL on entry to handler.  */
        const SA_RESTORER   =0x04000000;
        const SA_ALL = Self::SA_NOCLDSTOP.bits() |
            Self::SA_NOCLDWAIT.bits() |
            Self::SA_NODEFER.bits() |
            Self::SA_ONSTACK.bits() |
            Self::SA_RESETHAND.bits() |
            Self::SA_RESTART.bits() |
            Self::SA_SIGINFO.bits() |
            Self::SA_RESTORER.bits();
    }
}

// fixme: is this order correct?
#[derive(Clone, Copy)]
#[repr(C)]
pub struct USigAction {
    pub handler: usize,
    pub flags: SAFlags,
    pub restorer: usize,
    pub mask: SigMask,
}

#[derive(Clone, Copy, Debug)]
pub struct KSigAction {
    pub handler: SAHandlerType,
    pub mask: SigMask,
    pub flags: SAFlags,
    pub restorer: usize,
}

impl KSigAction {
    pub const fn new_default(sig: Signal) -> Self {
        Self {
            handler: SAHandlerType::new_default(sig),
            mask: SigMask::empty(),
            flags: SAFlags::empty(),
            restorer: 0,
        }
    }
}

impl KSigAction {
    pub fn from_sa(sa: USigAction, signum: Signal) -> Self {
        match sa.handler {
            SIG_DFL => KSigAction::new_default(signum),
            SIG_IGN => Self {
                handler: SAHandlerType::Ignore,
                flags: sa.flags,
                mask: sa.mask,
                restorer: sa.restorer,
            },
            handler => Self {
                handler: SAHandlerType::User { handler },
                flags: sa.flags,
                mask: sa.mask,
                restorer: sa.restorer,
            },
        }
    }

    pub fn into_sa(&self) -> USigAction {
        USigAction {
            handler: match self.handler {
                SAHandlerType::Ignore => SIG_IGN,
                SAHandlerType::Kill => SIG_DFL,
                SAHandlerType::Stop => SIG_DFL,
                SAHandlerType::Continue => SIG_DFL,
                SAHandlerType::User { handler } => handler,
            },
            flags: self.flags,
            mask: self.mask,
            restorer: 0,
        }
    }
}

/// signal action list of a task
#[derive(Clone)]
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
        debug!(
            "[SigActionList] set_sigaction: signum {:?}, action: {:?}, cur_bitmap: {:?}",
            Signal::from(signum),
            action,
            self.get_user_bitmap()
        );
    }
    pub fn get(&self, signum: Signal) -> Option<&KSigAction> {
        self.actions.get(signum as usize)
    }
    pub fn get_user_bitmap(&self) -> SigSet {
        let mut res = SigSet::empty();
        for (i, sa) in self.actions.iter().enumerate() {
            if let SAHandlerType::User { handler: _ } = sa.handler {
                res |= SigSet::from_signum(i as u32);
            }
        }
        res
    }
    pub fn get_ignored_bitmap(&self) -> SigSet {
        let mut res = SigSet::empty();
        for (i, sa) in self.actions.iter().enumerate() {
            if sa.handler == SAHandlerType::Ignore {
                res |= SigSet::from_signum(i as u32);
            }
        }
        res
    }
    pub fn reset(&mut self) {
        for (num, action) in self.actions.iter_mut().enumerate() {
            match action.handler {
                SAHandlerType::User { .. } => {
                    action.handler = SAHandlerType::new_default(Signal::from((num + 1) as usize))
                }
                _ => {}
            }
        }
    }
}
