//! get current time since the system booted

#![allow(unused)]

use riscv::register::time;

use super::timeval::TimeVal;
use crate::constant::time::*;

/// fetch time from `mtime` counter,
/// which records the number of clock cycles since the system booted
pub fn get_time() -> usize {
    time::read()
}

pub fn get_time_s() -> usize {
    get_time() / CLOCK_FREQ
}

pub fn get_time_ms() -> usize {
    get_time() / (CLOCK_FREQ / MSEC_PER_SEC)
}

pub fn get_time_us() -> usize {
    let time = get_time();
    const DIVISOR: usize = CLOCK_FREQ / USEC_PER_SEC;
    const REMAINDER: usize = CLOCK_FREQ % USEC_PER_SEC;
    let integral_part = time / DIVISOR;
    let fraction_part = time * REMAINDER / CLOCK_FREQ;
    integral_part + fraction_part
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
