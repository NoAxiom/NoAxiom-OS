use arch::TrapContext;

use super::{
    sig_detail::SigDetail,
    sig_num::{SigErrno, Signo},
    sig_set::SigMask,
};

pub struct SigContext {
    pub cx: TrapContext,
    pub mask: SigMask,
}

impl SigContext {
    // pub const fn empty() -> Self {
    //     Self {
    //         cx: TrapContext::empty(),
    //         mask: SigMask::empty(),
    //     }
    // }
    pub fn from_another(cx: &TrapContext, mask: SigMask) -> Self {
        Self {
            cx: cx.clone(),
            mask: mask.clone(),
        }
    }
}

/// signal code
/// when value <= 0, it means the signal is sent by user mode
#[derive(Copy, Debug, Clone)]
#[repr(i32)]
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
}

#[derive(Clone, Copy, Debug)]
pub struct SigInfo {
    pub signo: Signo,      // signal number
    pub code: SigCode,     // signal code
    pub errno: SigErrno,   // errno value
    pub detail: SigDetail, // detailed info
}
