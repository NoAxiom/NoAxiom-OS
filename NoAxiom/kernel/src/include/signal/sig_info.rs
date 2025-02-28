use arch::TrapContext;

use super::{
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

pub struct SigInfo {
    pub signo: Signo,             // signal number
    pub code: SigCode,            // signal code
    pub errno: SigErrno,          // errno value
    pub extra_info: SigExtraInfo, // extra info
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

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SigExtraInfo {
    Basic,
    Extend {
        si_pid: u32, // Sending process ID
        // si_uid: u32,         // Real user ID of sending process
        si_status: Option<i32>, // Exit value or signal
        si_utime: Option<i32>,  // User time consumed
        si_stime: Option<i32>,  // System time consumed
    },
}
