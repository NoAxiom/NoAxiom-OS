use bitflags::bitflags;

use super::{sig_num::SigNum, sig_set::SigMask};
use crate::constant::signal::SIG_DFL;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigActionHandlerType {
    Default,
    Ignore,
    Customized { addr: usize },
}

impl SigActionHandlerType {
    pub const fn default(sig: SigNum) -> Self {
        match sig {
            SigNum::SIGCHLD | SigNum::SIGURG | SigNum::SIGWINCH => Self::Ignore,
            _ => Self::Default,
        }
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct SigActionFlags: u32 {
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

/* signal.h
struct sigaction {
    void (*sa_handler)(int);
    void (*sa_sigaction)(int, siginfo_t *, void *);
    sigset_t sa_mask;
    int sa_flags;
    void (*sa_restorer)(void); // not used
}
*/

// fixme: is this order correct?
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SigAction {
    pub sa_handler: usize,
    pub sa_flags: SigActionFlags,
    pub sa_restorer: usize,
    pub sa_mask: SigMask,
}

impl SigAction {
    pub fn new() -> Self {
        Self {
            sa_handler: SIG_DFL,
            sa_flags: SigActionFlags::empty(),
            sa_restorer: 0,
            sa_mask: SigMask::empty(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct KernelSigAction {
    pub handler: SigActionHandlerType,
    pub mask: SigMask,
    pub flags: SigActionFlags,
}

impl KernelSigAction {
    pub const fn new() -> Self {
        Self {
            handler: SigActionHandlerType::Default,
            mask: SigMask::empty(),
            flags: SigActionFlags::empty(),
        }
    }
}

impl Default for KernelSigAction {
    fn default() -> Self {
        Self::new()
    }
}
