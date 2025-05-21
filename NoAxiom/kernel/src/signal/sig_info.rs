use super::{
    sig_detail::SigDetail,
    sig_num::{SigErrno, Signo},
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

#[derive(Clone, Copy, Debug)]
pub struct SigInfo {
    /// signal number
    pub signo: Signo,

    /// signal code
    pub code: SigCode,

    /// errno value
    pub errno: SigErrno,

    /// detailed info
    pub detail: SigDetail,
}

impl SigInfo {
    pub fn new_simple(signo: Signo, code: SigCode) -> Self {
        Self {
            signo,
            code,
            errno: 0,
            detail: SigDetail::None,
        }
    }
    pub fn new_detailed(signo: Signo, code: SigCode, errno: SigErrno, detail: SigDetail) -> Self {
        Self {
            signo,
            code,
            errno,
            detail,
        }
    }
}
