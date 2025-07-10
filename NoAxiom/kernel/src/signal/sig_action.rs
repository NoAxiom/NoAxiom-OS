use core::ops::{Index, IndexMut};

use array_init::array_init;
use bitflags::bitflags;

use super::{
    sig_set::{SigMask, SigSet},
    signal::Signal,
};
use crate::signal::signal::{NSIG, SIG_DFL, SIG_IGN};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SAHandlerType {
    Ignore,
    Kill,
    Stop,
    Continue,
    User { handler: usize }, // handler addr
}

impl SAHandlerType {
    pub const fn new_default(signal: Signal) -> Self {
        match signal {
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
    pub const fn new_default(signal: Signal) -> Self {
        Self {
            handler: SAHandlerType::new_default(signal),
            mask: SigMask::empty(),
            flags: SAFlags::empty(),
            restorer: 0,
        }
    }
}

impl KSigAction {
    pub fn from_sa(sa: USigAction, signal: Signal) -> Self {
        match sa.handler {
            SIG_DFL => KSigAction::new_default(signal),
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
    pub actions: [KSigAction; NSIG],
}

impl SigActionList {
    pub fn new() -> Self {
        Self {
            actions: array_init(|index| {
                let signal = unsafe { Signal::from_raw_sa_index(index) };
                let res = KSigAction::new_default(signal);
                res
            }),
        }
    }
    pub fn set_sigaction(&mut self, signal: Signal, action: KSigAction) {
        self[signal] = action;
        debug!(
            "[SigActionList] set_sigaction: signum {:?}, action: {:?}",
            signal, action
        );
    }
    pub fn get_ignored_bitmap(&self) -> SigSet {
        let mut res = SigSet::empty();
        for (index, sa) in self.actions.iter().enumerate() {
            if sa.handler == SAHandlerType::Ignore {
                let sigset = unsafe { SigSet::from_raw_sa_index(index) };
                res |= sigset;
            }
        }
        res
    }
    pub fn reset(&mut self) {
        for (index, action) in self.actions.iter_mut().enumerate() {
            match action.handler {
                SAHandlerType::User { .. } => {
                    let signal = unsafe { Signal::from_raw_sa_index(index) };
                    action.handler = SAHandlerType::new_default(signal);
                }
                _ => {}
            }
        }
    }
}

impl Index<Signal> for SigActionList {
    type Output = KSigAction;

    fn index(&self, index: Signal) -> &Self::Output {
        self.actions
            .get(index.into_sigaction_index())
            .expect("SigActionList index out of bounds")
    }
}

impl IndexMut<Signal> for SigActionList {
    fn index_mut(&mut self, index: Signal) -> &mut Self::Output {
        self.actions
            .get_mut(index.into_sigaction_index())
            .expect("SigActionList index out of bounds")
    }
}
