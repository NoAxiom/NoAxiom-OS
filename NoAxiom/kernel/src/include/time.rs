use core::{
    ops::{Add, Sub},
    time::Duration,
};

use arch::{Arch, ArchTime};

use crate::constant::time::{NSEC_PER_SEC, USEC_PER_SEC};

#[repr(C)]
pub struct TMS {
    /// user time
    pub tms_utime: isize,
    /// system time
    pub tms_stime: isize,
    /// user time of dead children
    pub tms_cutime: isize,
    /// system time of dead children
    pub tms_cstime: isize,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSpec {
    pub tv_sec: u64,  // 秒
    pub tv_nsec: u64, // 纳秒
}

impl TimeSpec {
    pub fn empty() -> Self {
        Self {
            tv_sec: 0,
            tv_nsec: 0,
        }
    }

    pub fn into_ticks(&self) -> usize {
        let freq = Arch::get_freq();
        self.tv_sec as usize * freq + self.tv_nsec as usize / NSEC_PER_SEC * freq
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }

    pub fn into_ns(&self) -> usize {
        self.tv_sec as usize * NSEC_PER_SEC + self.tv_nsec as usize
    }

    pub fn from_ticks(tiks: usize) -> Self {
        let freq = Arch::get_freq();
        let tv_sec = tiks / freq;
        let tv_nsec = (tiks % freq) * NSEC_PER_SEC / freq;
        Self {
            tv_sec: tv_sec as u64,
            tv_nsec: tv_nsec as u64,
        }
    }
}
