//! get current time since the system booted

#![allow(unused)]

use core::time::Duration;

use arch::{Arch, ArchTime};

use super::time_val::TimeVal;
use crate::constant::time::*;

/// fetch time from `mtime` counter,
/// which records the number of clock cycles since the system booted
#[inline(always)]
pub fn get_time() -> usize {
    Arch::get_time()
}

pub fn get_time_s() -> usize {
    get_time() / Arch::get_freq()
}

pub fn get_time_ms() -> usize {
    get_time() / (Arch::get_freq() / MSEC_PER_SEC)
}

pub fn get_time_us() -> usize {
    get_time() * USEC_PER_SEC / Arch::get_freq()
}

pub fn get_time_ns() -> usize {
    get_time() * (NSEC_PER_SEC / Arch::get_freq())
}

pub fn get_timeval() -> TimeVal {
    let freq = Arch::get_freq();
    let ticks = get_time();
    let sec = ticks / freq;
    let usec = (ticks % freq) * USEC_PER_SEC / freq;
    TimeVal { sec, usec }
}

pub fn get_time_duration() -> Duration {
    Duration::from_micros(get_time_us() as u64)
}
