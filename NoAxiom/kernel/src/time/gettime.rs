//! get current time since the system booted

#![allow(unused)]

use core::time::Duration;

use arch::{Arch, ArchTime};

use super::timeval::TimeVal;
use crate::constant::time::*;

/// fetch time from `mtime` counter,
/// which records the number of clock cycles since the system booted
#[inline(always)]
pub fn get_time() -> usize {
    Arch::get_time()
}

pub fn get_time_s() -> usize {
    get_time() / CLOCK_FREQ
}

pub fn get_time_ms() -> usize {
    get_time() / (CLOCK_FREQ / MSEC_PER_SEC)
}

pub fn get_time_us() -> usize {
    get_time() * USEC_PER_SEC / CLOCK_FREQ
}

pub fn get_time_ns() -> usize {
    get_time() * (NSEC_PER_SEC / CLOCK_FREQ)
}

pub fn get_timeval() -> TimeVal {
    let ticks = get_time();
    let sec = ticks / CLOCK_FREQ;
    let usec = (ticks % CLOCK_FREQ) * USEC_PER_SEC / CLOCK_FREQ;
    TimeVal { sec, usec }
}

pub fn get_time_duration() -> Duration {
    Duration::from_micros(get_time_us() as u64)
}
