use core::time::Duration;

use arch::{Arch, ArchTime};
use config::sched::TIME_SLICE_PER_SEC;

use crate::constant::time::NSEC_PER_SEC;

const NSEC_PER_SLICE: usize = NSEC_PER_SEC / TIME_SLICE_PER_SEC; // 10ms
pub const TIME_SLICE_DURATION: Duration = Duration::from_nanos(NSEC_PER_SLICE as u64);

fn get_time_slice_ticks() -> usize {
    Arch::get_freq() / TIME_SLICE_PER_SEC
}

pub struct TimeSliceInfo {
    ticks: u64,
}

impl Default for TimeSliceInfo {
    fn default() -> Self {
        Self::normal()
    }
}

impl TimeSliceInfo {
    pub fn ticks(&self) -> u64 {
        self.ticks
    }
    pub fn realtime() -> Self {
        Self {
            ticks: get_time_slice_ticks() as u64 / 8, // 1.25ms
        }
    }
    pub fn normal() -> Self {
        Self {
            ticks: get_time_slice_ticks() as u64, // 10ms
        }
    }
    pub fn infinite() -> Self {
        Self {
            ticks: u32::MAX as u64, // no time slice
        }
    }
}

/// set next timer interrupt by time_slice
pub fn set_next_trigger(info: Option<TimeSliceInfo>) {
    Arch::set_timer(info.unwrap_or_default().ticks() as u64);
}
