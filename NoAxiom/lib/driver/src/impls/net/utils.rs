use arch::{Arch, ArchTime};
use smoltcp::time::Instant;

pub fn get_time_ms() -> usize {
    const MSEC_PER_SEC: usize = 1000;
    arch::Arch::get_time() / (Arch::get_freq() / MSEC_PER_SEC)
}

pub fn get_time_instant() -> Instant {
    Instant::from_millis(get_time_ms() as i64)
}
