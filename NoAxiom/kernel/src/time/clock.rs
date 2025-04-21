use alloc::collections::btree_map::BTreeMap;
use core::time::Duration;

use ksync::mutex::SpinLock;
use strum::FromRepr;

#[allow(non_camel_case_types)]
#[repr(usize)]
#[derive(FromRepr, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClockId {
    CLOCK_REALTIME = 0,
    CLOCK_MONOTONIC = 1,
    CLOCK_PROCESS_CPUTIME_ID = 2,
    CLOCK_THREAD_CPUTIME_ID = 3,
    CLOCK_MONOTONIC_RAW = 4,
    CLOCK_REALTIME_COARSE = 5,
    CLOCK_MONOTONIC_COARSE = 6,
}

/// clock stores the deviation: arg time - dev time(current_time)
pub struct ClockManager(pub BTreeMap<ClockId, Duration>);

/// Clock manager that used for looking for a given process
pub static CLOCK_MANAGER: SpinLock<ClockManager> = SpinLock::new(ClockManager(BTreeMap::new()));

pub fn ktime_init() {
    // todo: currently all the clocks in ClockManager
    // are zero-inited, consider provide a real clock

    // CLOCK_REALTIME: 0
    CLOCK_MANAGER
        .lock()
        .0
        .insert(ClockId::CLOCK_REALTIME, Duration::ZERO);

    // CLOCK_MONOTONIC: 1
    CLOCK_MANAGER
        .lock()
        .0
        .insert(ClockId::CLOCK_MONOTONIC, Duration::ZERO);

    // CLOCK_MONOTONIC_RAW: 4
    CLOCK_MANAGER
        .lock()
        .0
        .insert(ClockId::CLOCK_MONOTONIC_RAW, Duration::ZERO);

    // CLOCK_REALTIME_COARSE: 5
    CLOCK_MANAGER
        .lock()
        .0
        .insert(ClockId::CLOCK_REALTIME_COARSE, Duration::ZERO);

    // CLOCK_MONOTONIC_COARSE: 6
    CLOCK_MANAGER
        .lock()
        .0
        .insert(ClockId::CLOCK_MONOTONIC_COARSE, Duration::ZERO);
}
