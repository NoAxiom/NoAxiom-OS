use include::errno::{Errno, SysResult};
use strum::FromRepr;

use super::sig_set::SigMask;
use crate::return_errno;

pub const NSIG: usize = 64;

// The SIG_DFL and SIG_IGN macros expand into integral expressions that are not
// equal to an address of any function. The macros define signal handling
// strategies for signal() function.
pub const SIG_DFL: usize = 0; // default signal handling
pub const SIG_IGN: usize = 1; // signal is ignored

pub type SigErrno = i32;

#[derive(PartialEq, Eq, Copy, Clone, Debug, FromRepr)]
#[repr(usize)]
#[allow(non_camel_case_types, unused)]
pub enum Signal {
    SIGINVAL = 0, // invalid signal, used for error handling
    // non-rt signal
    SIGHUP = 1,
    SIGINT = 2,
    SIGQUIT = 3,
    SIGILL = 4,
    SIGTRAP = 5,
    SIGABRT = 6, // a.k.a SIGIOT
    SIGBUS = 7,
    SIGFPE = 8,
    SIGKILL = 9,
    SIGUSR1 = 10,

    SIGSEGV = 11,
    SIGUSR2 = 12,
    SIGPIPE = 13,
    SIGALRM = 14,
    SIGTERM = 15,
    SIGSTKFLT = 16,
    SIGCHLD = 17,
    SIGCONT = 18,
    SIGSTOP = 19,
    SIGTSTP = 20,

    SIGTTIN = 21,
    SIGTTOU = 22,
    SIGURG = 23,
    SIGXCPU = 24,
    SIGXFSZ = 25,
    SIGVTALRM = 26,
    SIGPROF = 27,
    SIGWINCH = 28,
    SIGIO = 29, // a.k.a SIGPOLL
    SIGPWR = 30,
    SIGSYS = 31, // a.k.a SIGUNUSED

    // rt signal, not implemented yet!
    SIGTIMER = 32,
    SIGCANCEL = 33,
    SIGSYNCCALL = 34,
    SIGRT_3 = 35,
    SIGRT_4 = 36,
    SIGRT_5 = 37,
    SIGRT_6 = 38,
    SIGRT_7 = 39,
    SIGRT_8 = 40,
    SIGRT_9 = 41,
    SIGRT_10 = 42,
    SIGRT_11 = 43,
    SIGRT_12 = 44,
    SIGRT_13 = 45,
    SIGRT_14 = 46,
    SIGRT_15 = 47,
    SIGRT_16 = 48,
    SIGRT_17 = 49,
    SIGRT_18 = 50,
    SIGRT_19 = 51,
    SIGRT_20 = 52,
    SIGRT_21 = 53,
    SIGRT_22 = 54,
    SIGRT_23 = 55,
    SIGRT_24 = 56,
    SIGRT_25 = 57,
    SIGRT_26 = 58,
    SIGRT_27 = 59,
    SIGRT_28 = 60,
    SIGRT_29 = 61,
    SIGRT_30 = 62,
    SIGRT_31 = 63,
    SIGRTMAX = 64,
}

impl Signal {
    pub unsafe fn from_raw_sa_index(index: usize) -> Self {
        Self::from_repr(index + 1).unwrap()
    }
    #[inline]
    pub fn raw(self) -> usize {
        self as usize
    }
    pub fn into_sigmask_offset(self) -> usize {
        self.raw() - 1
    }
    pub fn into_sigaction_index(self) -> usize {
        self.raw() - 1
    }
    pub fn try_exclude_kill(self) -> SysResult<Self> {
        match self {
            Signal::SIGKILL | Signal::SIGSTOP => {
                return_errno!(Errno::EINVAL, "Cannot register SIGKILL or SIGSTOP");
            }
            _ => Ok(self),
        }
    }
}

impl TryFrom<usize> for Signal {
    type Error = Errno;

    fn try_from(signo: usize) -> SysResult<Self> {
        if let Some(signal) = Self::from_repr(signo) {
            Ok(signal)
        } else {
            Err(Errno::EINVAL)
        }
    }
}

impl Signal {
    /// returns None if signo is zero
    /// otherwise returns Some(Ok(signal)) if signo is valid, or
    /// Some(Err(Errno::EINVAL)) if invalid
    pub fn try_from_with_zero_as_none(signo: usize) -> SysResult<Option<Self>> {
        if signo == 0 {
            Ok(None)
        } else {
            if let Some(signal) = Self::from_repr(signo) {
                Ok(Some(signal))
            } else {
                Err(Errno::EINVAL)
            }
        }
    }
}

impl TryInto<SigMask> for Signal {
    type Error = Errno;
    fn try_into(self) -> SysResult<SigMask> {
        let offset = self.into_sigmask_offset();
        if offset >= NSIG {
            return_errno!(
                Errno::EINVAL,
                "invalid signum when converting signal {:?} to SigMask",
                self
            );
        }
        Ok(SigMask::from_bits_truncate(1 << offset))
    }
}
