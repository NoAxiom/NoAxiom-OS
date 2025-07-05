pub const MAX_SIGNUM: u32 = 64;

// The SIG_DFL and SIG_IGN macros expand into integral expressions that are not
// equal to an address of any function. The macros define signal handling
// strategies for signal() function.
pub const SIG_DFL: usize = 0; // default signal handling
pub const SIG_IGN: usize = 1; // signal is ignored

pub type Signo = i32;
pub type SigErrno = i32;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[repr(usize)]
#[allow(non_camel_case_types, unused)]
pub enum Signal {
    // empty signal, it means no signal
    INVALID = 0,

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

// signum <> usize
impl From<usize> for Signal {
    fn from(value: usize) -> Self {
        if value < MAX_SIGNUM as usize {
            let ret: Signal = unsafe { core::mem::transmute(value) };
            return ret;
        } else {
            error!("[SIGNAL] Try to convert an invalid number to Signal");
            return Signal::INVALID;
        }
    }
}
impl Into<usize> for Signal {
    fn into(self) -> usize {
        self as usize
    }
}

// signum <> signo(i32)
impl Into<Signo> for Signal {
    fn into(self) -> Signo {
        self as Signo
    }
}
impl From<Signo> for Signal {
    fn from(value: Signo) -> Self {
        if value < 0 {
            error!("[SIGNAL] Try to convert an invalid number to Signal");
            return Signal::INVALID;
        } else {
            return Self::from(value as usize);
        }
    }
}
