use super::gettime::get_time;
use crate::{config::sched::TIME_SLICE_TICKS, cpu::get_hartid, driver::sbi::set_timer};

/// set next timer interrupt by time_slice
/// todo: add variable time slice
pub fn set_next_trigger() {
    // let current_time = get_time();
    // current_cpu().set_time(current_time);
    trace!(
        "[set_next_trigger] hart: {}, cur: {}, to: {}",
        get_hartid(),
        get_time(),
        (get_time() + TIME_SLICE_TICKS) as u64
    );
    set_timer((get_time() + TIME_SLICE_TICKS) as u64);
}

// /// clear timer interrupt by setting an unreachable stime_value
// pub fn clear_trigger() {
//     set_timer(u64::MAX);
// }
