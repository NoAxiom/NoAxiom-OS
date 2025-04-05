use arch::{Arch, ArchTime};
use config::sched::{LOAD_BALANCE_SLICE_NUM, TIME_SLICE_PER_SEC};

pub fn get_time_slice_ticks() -> usize {
    Arch::get_freq() / TIME_SLICE_PER_SEC
}

/// set next timer interrupt by time_slice
/// todo: add variable time slice
pub fn set_next_trigger() {
    Arch::set_timer(get_time_slice_ticks() as u64);
}

pub fn get_load_balance_ticks() -> usize {
    get_time_slice_ticks() * LOAD_BALANCE_SLICE_NUM
}
