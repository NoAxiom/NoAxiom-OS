use bitflags::bitflags;

use super::{sig_num::SigNum, sig_set::SigMask};

// The SIG_DFL and SIG_IGN macros expand into integral expressions that are not
// equal to an address of any function. The macros define signal handling
// strategies for signal() function.
pub const SIG_DFL: usize = 0; // default signal handling
pub const SIG_IGN: usize = 1; // signal is ignored

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaHandlerType {
    Default,
    Ignore,
    Customized { addr: usize },
}

impl SaHandlerType {
    pub const fn default(sig: SigNum) -> Self {
        match sig {
            SigNum::SIGCHLD | SigNum::SIGURG | SigNum::SIGWINCH => Self::Ignore,
            _ => Self::Default,
        }
    }
}

bitflags! {
    #[derive(Copy, Clone, Debug)]
    pub struct SaFlags: u32 {
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

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SigAction {
    pub sa_handler: usize,
    _sa_sigaction: usize,
    pub sa_mask: SigMask,
    pub sa_flags: SaFlags,
    _sa_restorer: usize,
}

impl SigAction {
    pub fn new() -> Self {
        Self {
            sa_handler: SIG_DFL,
            _sa_sigaction: 0,
            sa_mask: SigMask::empty(),
            sa_flags: SaFlags::empty(),
            _sa_restorer: 0,
        }
    }
}

pub struct KernelSigAction {
    pub handler: SaHandlerType,
    pub mask: SigMask,
    pub flags: SaFlags,
}

impl KernelSigAction {
    pub fn new() -> Self {
        Self {
            handler: SaHandlerType::Default,
            mask: SigMask::empty(),
            flags: SaFlags::empty(),
        }
    }
}

impl From<SigAction> for KernelSigAction {
    fn from(sa: SigAction) -> Self {
        Self {
            handler: match sa.sa_handler {
                SIG_DFL => SaHandlerType::Default,
                SIG_IGN => SaHandlerType::Ignore,
                addr => SaHandlerType::Customized { addr },
            },
            mask: sa.sa_mask,
            flags: sa.sa_flags,
        }
    }
}

impl From<KernelSigAction> for SigAction {
    fn from(ksa: KernelSigAction) -> Self {
        Self {
            sa_handler: match ksa.handler {
                SaHandlerType::Default => SIG_DFL,
                SaHandlerType::Ignore => SIG_IGN,
                SaHandlerType::Customized { addr } => addr,
            },
            _sa_sigaction: 0,
            sa_mask: ksa.mask,
            sa_flags: ksa.flags,
            _sa_restorer: 0,
        }
    }
}
