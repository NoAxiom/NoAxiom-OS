use bitflags::bitflags;

use super::{sig_num::SigNum, sig_set::SigMask};
use crate::constant::signal::{SIG_DFL, SIG_IGN};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SAHandlerType {
    Ignore,
    Kill,
    Stop,
    Continue,
    User { handler: usize }, // handler addr
}

impl SAHandlerType {
    pub fn new_default(sig: SigNum) -> Self {
        match sig {
            SigNum::SIGCHLD | SigNum::SIGURG | SigNum::SIGWINCH => Self::Ignore,
            SigNum::SIGSTOP | SigNum::SIGTSTP | SigNum::SIGTTIN | SigNum::SIGTTOU => Self::Stop,
            SigNum::SIGCONT => Self::Continue,
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
pub struct SigAction {
    pub handler: usize,
    pub flags: SAFlags,
    pub restorer: usize,
    pub mask: SigMask,
}

#[derive(Clone, Copy)]
pub struct KSigAction {
    pub handler: SAHandlerType,
    pub mask: SigMask,
    pub flags: SAFlags,
}

impl KSigAction {
    pub fn new_default(sig: SigNum) -> Self {
        Self {
            handler: SAHandlerType::new_default(sig),
            mask: SigMask::empty(),
            flags: SAFlags::empty(),
        }
    }
}

impl KSigAction {
    pub fn from_sa(sa: SigAction, signum: SigNum) -> Self {
        match sa.handler {
            SIG_DFL => KSigAction::new_default(signum),
            SIG_IGN => Self {
                handler: SAHandlerType::Ignore,
                flags: sa.flags,
                mask: sa.mask,
            },
            handler => Self {
                handler: SAHandlerType::User { handler },
                flags: sa.flags,
                mask: sa.mask,
            },
        }
    }
    
    pub fn into_sa(&self) -> SigAction {
        SigAction {
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
