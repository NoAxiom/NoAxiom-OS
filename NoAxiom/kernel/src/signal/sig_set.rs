use alloc::string::String;

use bitflags::bitflags;
use include::errno::{Errno, SysResult};

use super::signal::Signal;
use crate::{return_errno, signal::signal::NSIG};

pub type SigSetReprType = usize;

bitflags! {
    /// Signal mask
    /// to record which signal is blocked
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
    pub struct SigSet: SigSetReprType {
        const SIGHUP      = 1 << 0;
        const SIGINT      = 1 << 1;
        const SIGQUIT     = 1 << 2;
        const SIGILL      = 1 << 3;
        const SIGTRAP     = 1 << 4;
        const SIGABRT     = 1 << 5;
        const SIGBUS      = 1 << 6;
        const SIGFPE      = 1 << 7;
        const SIGKILL     = 1 << 8;
        const SIGUSR1     = 1 << 9;
        const SIGSEGV     = 1 << 10;
        const SIGUSR2     = 1 << 11;
        const SIGPIPE     = 1 << 12;
        const SIGALRM     = 1 << 13;
        const SIGTERM     = 1 << 14;
        const SIGSTKFLT   = 1 << 15;
        const SIGCHLD     = 1 << 16;
        const SIGCONT     = 1 << 17;
        const SIGSTOP     = 1 << 18;
        const SIGTSTP     = 1 << 19;
        const SIGTTIN     = 1 << 20;
        const SIGTTOU     = 1 << 21;
        const SIGURG      = 1 << 22;
        const SIGXCPU     = 1 << 23;
        const SIGXFSZ     = 1 << 24;
        const SIGVTALRM   = 1 << 25;
        const SIGPROF     = 1 << 26;
        const SIGWINCH    = 1 << 27;
        const SIGIO       = 1 << 28;
        const SIGPWR      = 1 << 29;
        const SIGSYS      = 1 << 30;
        const SIGTIMER    = 1 << 31;
        const SIGCANCEL   = 1 << 32;
        const SIGSYNCCALL = 1 << 33;
        const SIGRT_3     = 1 << 34;
        const SIGRT_4     = 1 << 35;
        const SIGRT_5     = 1 << 36;
        const SIGRT_6     = 1 << 37;
        const SIGRT_7     = 1 << 38;
        const SIGRT_8     = 1 << 39;
        const SIGRT_9     = 1 << 40;
        const SIGRT_10    = 1 << 41;
        const SIGRT_11    = 1 << 42;
        const SIGRT_12    = 1 << 43;
        const SIGRT_13    = 1 << 44;
        const SIGRT_14    = 1 << 45;
        const SIGRT_15    = 1 << 46;
        const SIGRT_16    = 1 << 47;
        const SIGRT_17    = 1 << 48;
        const SIGRT_18    = 1 << 49;
        const SIGRT_19    = 1 << 50;
        const SIGRT_20    = 1 << 51;
        const SIGRT_21    = 1 << 52;
        const SIGRT_22    = 1 << 53;
        const SIGRT_23    = 1 << 54;
        const SIGRT_24    = 1 << 55;
        const SIGRT_25    = 1 << 56;
        const SIGRT_26    = 1 << 57;
        const SIGRT_27    = 1 << 58;
        const SIGRT_28    = 1 << 59;
        const SIGRT_29    = 1 << 60;
        const SIGRT_30    = 1 << 61;
        const SIGRT_31    = 1 << 62;
        const SIGRTMAX    = 1 << NSIG - 1;

        const BLOCKED = Self::SIGKILL.bits() | Self::SIGSTOP.bits();
    }
}

pub type SigMask = SigSet;

impl SigSet {
    pub fn without_kill(self) -> Self {
        self - Self::BLOCKED
    }
    pub fn with_kill(self) -> Self {
        self | Self::BLOCKED
    }
    pub fn enable(&mut self, signal: Signal) {
        let _ = self.enable_checked(signal);
    }
    pub fn enable_checked(&mut self, signal: Signal) -> SysResult<()> {
        let signo = signal.into_raw_signo();
        if signo >= NSIG {
            return_errno!(
                Errno::EINVAL,
                "invalid signum when enable signal {:?}",
                signal,
            );
        }
        *self |= SigSet::from_bits_truncate(1 << signo);
        Ok(())
    }
    pub fn disable(&mut self, signal: Signal) -> SysResult<()> {
        let signo = signal.into_raw_signo();
        if signo >= NSIG {
            return_errno!(
                Errno::EINVAL,
                "invalid signum when disable signum {}",
                signo
            );
        }
        *self -= SigSet::from_bits_truncate(1 << signo);
        Ok(())
    }
    pub fn contains_signal(&self, signal: Signal) -> bool {
        let signo = signal.into_raw_signo();
        self.contains(SigSet::from_bits_truncate(1 << signo))
    }
    pub unsafe fn from_raw_signo(index: usize) -> Self {
        SigSet::from_bits_truncate(1 << index)
    }
    pub fn debug_info_short(&self) -> String {
        let mask_size = self.bits().count_ones();
        let rev = !*self;
        let rev_size = rev.bits().count_ones();
        if mask_size == 0 {
            String::from("Empty")
        } else if rev_size == 0 {
            String::from("Full")
        } else if mask_size < rev_size {
            format!("{:?}", *self)
        } else {
            format!("Skip[{:?}]", rev)
        }
    }
}
