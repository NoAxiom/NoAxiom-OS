use super::{
    sig_detail::SigDetail,
    signal::{SigErrno, Signal},
};

/// signal code
/// when value <= 0, it means the signal is sent by user mode
#[derive(Copy, Debug, Clone)]
#[repr(i32)]
#[allow(unused)]
pub enum SigCode {
    /// sent by kill, sigsend, raise
    User = 0,
    /// sent by kernel from somewhere
    Kernel = 0x80,
    /// sent by sigqueue
    Queue = -1,
    /// send when timer expired
    Timer = -2,
    /// send when realtime messsage queue state change
    Mesgq = -3,
    /// send when async IO completed
    AsyncIO = -4,
    /// sent by queued SIGIO
    SigIO = -5,
    /// sent by tkill system call
    TKill = -6,
}

#[allow(unused)]
pub struct RawSigInfo {
    pub signo: usize,
    pub code: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct SigInfo {
    /// signal number
    pub signal: Signal,

    /// signal code
    pub code: SigCode,

    /// errno value
    pub errno: SigErrno,

    /// detailed info
    pub detail: SigDetail,
}

impl SigInfo {
    pub fn new_simple(signal: Signal, code: SigCode) -> Self {
        Self {
            signal,
            code,
            errno: 0,
            detail: SigDetail::None,
        }
    }
    pub fn new_detailed(signal: Signal, code: SigCode, errno: SigErrno, detail: SigDetail) -> Self {
        Self {
            signal,
            code,
            errno,
            detail,
        }
    }
    pub fn into_raw(self) -> RawSigInfo {
        RawSigInfo {
            signo: self.signal.into_raw_signo(),
            code: self.code as i32,
        }
    }
}
