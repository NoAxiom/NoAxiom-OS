#![allow(unused)]

use core::time::Duration;

use super::{gettime::get_time_duration, time_slice::TIME_SLICE_DURATION};
use crate::include::time::TMS;

#[derive(Debug, Clone, Copy)]
pub struct KernelDuration {
    /// user time
    pub utime: Duration,
    /// system time
    pub stime: Duration,
}
impl KernelDuration {
    pub const fn zero() -> Self {
        Self {
            utime: Duration::ZERO,
            stime: Duration::ZERO,
        }
    }
    #[inline(always)]
    pub fn add(&mut self, other: KernelDuration) {
        self.utime += other.utime;
        self.stime += other.stime;
    }
    #[inline(always)]
    pub fn cpu_time(&self) -> Duration {
        self.utime + self.stime
    }
}
impl Default for KernelDuration {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimeInfo {
    time: KernelDuration,
    child_time: KernelDuration,
    create_time: Duration,
    system_time_start: Duration,
    user_time_start: Duration,
    schedule_time_start: Duration,
}

impl TimeInfo {
    pub fn new() -> Self {
        Self {
            time: KernelDuration::zero(),
            child_time: KernelDuration::zero(),
            create_time: get_time_duration(),
            system_time_start: Duration::ZERO,
            user_time_start: Duration::ZERO,
            schedule_time_start: Duration::ZERO,
        }
    }
    #[inline(always)]
    pub const fn time(&self) -> KernelDuration {
        self.time
    }
    #[inline(always)]
    pub fn cpu_time(&self) -> Duration {
        self.time.cpu_time()
    }
    #[inline(always)]
    pub const fn child_time(&self) -> KernelDuration {
        self.child_time
    }
    #[inline(always)]
    pub const fn utime(&self) -> Duration {
        self.time.utime
    }
    #[inline(always)]
    pub const fn stime(&self) -> Duration {
        self.time.stime
    }
    #[inline(always)]
    pub const fn create_time(&self) -> Duration {
        self.create_time
    }

    pub fn into_tms(self) -> TMS {
        TMS {
            tms_utime: self.time.utime.as_micros() as usize,
            tms_stime: self.time.stime.as_micros() as usize,
            tms_cutime: self.child_time.utime.as_micros() as usize,
            tms_cstime: self.child_time.stime.as_micros() as usize,
        }
    }

    #[inline(always)]
    pub fn add_stime(&mut self, stime: Duration) {
        self.time.stime += stime;
    }
    #[inline(always)]
    pub fn add_utime(&mut self, utime: Duration) {
        self.time.utime += utime;
    }
    #[inline(always)]
    pub fn add_time(&mut self, time: KernelDuration) {
        self.time.add(time);
    }
    #[inline(always)]
    pub fn add_child_time(&mut self, time: KernelDuration) {
        self.child_time.add(time);
    }

    pub fn record_switch_in(&mut self) {
        let current_time = get_time_duration();
        self.system_time_start = current_time;
        self.schedule_time_start = current_time;
    }
    pub fn record_switch_out(&mut self) {
        let stime_slice = get_time_duration() - self.system_time_start;
        self.add_stime(stime_slice);
    }
    pub fn record_trap_out(&mut self) {
        let current_time = get_time_duration();
        self.system_time_start = current_time;
        let utime_slice = current_time - self.user_time_start;
        self.add_utime(utime_slice);
    }
    pub fn record_trap_in(&mut self) {
        let current_time = get_time_duration();
        let stime_slice = current_time - self.user_time_start;
        if self.user_time_start != Duration::ZERO {
            self.add_stime(stime_slice);
        }
        self.user_time_start = current_time;
    }
    pub fn is_timeup(&self) -> bool {
        get_time_duration() - self.schedule_time_start >= TIME_SLICE_DURATION
    }
}

impl Default for TimeInfo {
    fn default() -> Self {
        Self::new()
    }
}
