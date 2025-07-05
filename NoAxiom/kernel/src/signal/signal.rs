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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signo(i32);
impl Signo {
    #[inline]
    pub fn new(value: i32) -> Self {
        Self(value)
    }
    #[inline]
    pub fn raw_usize(self) -> usize {
        self.0 as usize
    }
    #[inline]
    pub fn raw_i32(self) -> i32 {
        self.0
    }
    #[inline]
    pub fn raw_isize(self) -> isize {
        self.0 as isize
    }
    /// convert to Signal enum index
    #[inline]
    pub fn into_signal_id(self) -> usize {
        self.raw_usize() + 1
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, FromRepr)]
#[repr(usize)]
#[allow(non_camel_case_types, unused)]
pub enum Signal {
    // non-rt signal
    SIGHUP = 1,
    SIGINT = 2,
    SIGQUIT = 3,
    SIGILL = 4,
    SIGTRAP = 5,
    SIGABRT = 6,
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
    SIGIO = 29,
    SIGPWR = 30,
    SIGSYS = 31,

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
    pub unsafe fn from_raw_signo_unchecked(raw: usize) -> Self {
        Self::from_repr(raw + 1).unwrap()
    }
    #[inline]
    pub fn into_signo(self) -> Signo {
        self.into()
    }
    #[inline]
    pub fn into_raw_signo(self) -> usize {
        self.into_signo().raw_usize()
    }
    #[inline]
    pub fn into_sigmask(self) -> SigMask {
        self.into()
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

impl TryFrom<Signo> for Signal {
    type Error = Errno;

    fn try_from(signo: Signo) -> SysResult<Self> {
        if let Some(signal) = Self::from_repr(signo.into_signal_id()) {
            Ok(signal)
        } else {
            return_errno!(
                Errno::EINVAL,
                "[SIGNAL] Try to convert an invalid number to Signal"
            );
        }
    }
}

impl Into<SigMask> for Signal {
    fn into(self) -> SigMask {
        let signo = self.into_raw_signo();
        if signo >= NSIG {
            panic!("invalid signal number when converting to SigMask");
        }
        SigMask::from_bits_truncate(1 << signo)
    }
}

impl Into<Signo> for Signal {
    #[inline]
    fn into(self) -> Signo {
        Signo::new(self as i32 - 1)
    }
}
