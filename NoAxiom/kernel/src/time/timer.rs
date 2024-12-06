use super::gettime::get_time;
use crate::{config::sched::TIME_SLICE_TICKS, driver::sbi::set_timer};

/// todo: add variable time slice
pub fn set_next_trigger() {
    // let current_time = get_time();
    // current_cpu().set_time(current_time);
    set_timer((get_time() + TIME_SLICE_TICKS) as u64);
}
