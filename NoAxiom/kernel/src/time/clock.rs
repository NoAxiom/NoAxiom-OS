use alloc::collections::btree_map::BTreeMap;
use core::time::Duration;

use ksync::mutex::SpinLock;

pub const CLOCK_REALTIME: usize = 0;
pub const CLOCK_MONOTONIC: usize = 1;
pub const CLOCK_PROCESS_CPUTIME_ID: usize = 2;

/// clock stores the deviation: arg time - dev time(current_time)
pub struct ClockManager(pub BTreeMap<usize, Duration>);

/// Clock manager that used for looking for a given process
pub static CLOCK_MANAGER: SpinLock<ClockManager> = SpinLock::new(ClockManager(BTreeMap::new()));

pub fn ktime_init() {
    CLOCK_MANAGER
        .lock()
        .0
        .insert(CLOCK_MONOTONIC, Duration::ZERO);
    CLOCK_MANAGER
        .lock()
        .0
        .insert(CLOCK_REALTIME, Duration::ZERO);
}
