use core::{
    ops::{Add, Sub},
    time::Duration,
};

use arch::{Arch, ArchTime};
use strum::FromRepr;

use crate::constant::time::USEC_PER_SEC;

/// Describes times in seconds and nanoseconds.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct TimeSpec {
    pub tv_sec: usize,
    pub tv_nsec: usize,
}

impl TimeSpec {
    pub fn into_ms(&self) -> usize {
        self.tv_sec * 1_000 + self.tv_nsec / 1_000_000
    }

    pub fn from_ms(ms: usize) -> Self {
        Self {
            tv_sec: ms / 1000,
            tv_nsec: (ms % 1000) * 1_000_000,
        }
    }

    pub fn is_valid(&self) -> bool {
        (self.tv_sec as isize >= 0)
            && (self.tv_nsec as isize >= 0)
            && (self.tv_nsec < 1_000_000_000)
    }

    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl From<Duration> for TimeSpec {
    fn from(duration: Duration) -> Self {
        Self {
            tv_sec: duration.as_secs() as usize,
            tv_nsec: duration.subsec_nanos() as usize,
        }
    }
}

impl From<TimeSpec> for Duration {
    fn from(time_spec: TimeSpec) -> Self {
        Duration::new(time_spec.tv_sec as u64, time_spec.tv_nsec as u32)
    }
}

#[repr(C)]
pub struct TMS {
    /// user time
    pub tms_utime: usize,
    /// system time
    pub tms_stime: usize,
    /// user time of dead children
    pub tms_cutime: usize,
    /// system time of dead children
    pub tms_cstime: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

impl From<TimeVal> for Duration {
    fn from(time_val: TimeVal) -> Self {
        Duration::new(time_val.sec as u64, (time_val.usec * 1000) as u32)
    }
}

impl From<Duration> for TimeVal {
    fn from(duration: Duration) -> Self {
        Self {
            sec: duration.as_secs() as usize,
            usec: duration.subsec_micros() as usize,
        }
    }
}
impl core::fmt::Display for TimeVal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}s {}us", self.sec, self.usec)
    }
}

impl Add for TimeVal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut sec = self.sec + other.sec;
        let mut usec = self.usec + other.usec;
        sec += usec / USEC_PER_SEC;
        usec %= USEC_PER_SEC;
        Self { sec, usec }
    }
}

impl Sub for TimeVal {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        if self.sec < other.sec {
            return Self { sec: 0, usec: 0 };
        } else if self.sec == other.sec {
            if self.usec < other.usec {
                return Self { sec: 0, usec: 0 };
            } else {
                return Self {
                    sec: 0,
                    usec: self.usec - other.usec,
                };
            }
        } else {
            let mut sec = self.sec - other.sec;
            let usec = if self.usec < other.usec {
                sec -= 1;
                USEC_PER_SEC + self.usec - other.usec
            } else {
                self.usec - other.usec
            };
            Self { sec, usec }
        }
    }
}

impl TimeVal {
    pub fn new() -> Self {
        Self { sec: 0, usec: 0 }
    }

    #[inline]
    pub fn zero() -> Self {
        Self { sec: 0, usec: 0 }
    }

    pub fn is_zero(&self) -> bool {
        self.sec == 0 && self.usec == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }

    pub fn from_ticks(tiks: usize) -> Self {
        let freq = Arch::get_freq();
        let sec = tiks / freq;
        let usec = (tiks % freq) * USEC_PER_SEC / freq;
        Self { sec, usec }
    }

    pub fn into_ticks(&self) -> usize {
        let freq = Arch::get_freq();
        self.sec * freq + self.usec / USEC_PER_SEC * freq
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ITimerVal {
    /// timer interval for periodic timer
    pub it_interval: TimeVal,
    /// time until next expiration
    pub it_value: TimeVal,
}

/// the size of interval timer is 3
pub const ITIMER_COUNT: usize = 3;

#[repr(usize)]
#[derive(FromRepr)]
pub enum ITimerType {
    /// real time timer
    Real = 0,
    /// virtual time timer
    Virtual = 1,
    /// profiling timer
    Prof = 2,
}
