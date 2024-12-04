use crate::{config::sched::TIME_SLICE_TICKS, driver::sbi::set_timer};

use super::gettime::get_time;

/// todo: add variable time slice
pub fn set_next_trigger() {
    set_timer((get_time() + TIME_SLICE_TICKS) as u64);
}
