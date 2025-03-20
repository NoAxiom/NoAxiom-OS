use arch::{Arch, ArchTime};

use crate::config::sched::TIME_SLICE_TICKS;

/// set next timer interrupt by time_slice
/// todo: add variable time slice
pub fn set_next_trigger() {
    Arch::set_timer(TIME_SLICE_TICKS as u64);
}
