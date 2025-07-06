use alloc::collections::btree_map::BTreeMap;
use core::{time::Duration, usize};

use ksync::mutex::SpinLock;

use crate::time::gettime::get_time_duration;

#[allow(unused)]
/// Frequency division
pub struct IntermitCrossover {
    cnt: usize,
    cnt_interval: usize,
    last_time: Duration,
    time_interval: Duration,
}

#[allow(unused)]
impl IntermitCrossover {
    pub fn new(cnt_interval: usize, time_interval: Duration) -> Self {
        IntermitCrossover {
            cnt: cnt_interval,
            cnt_interval,
            last_time: Duration::ZERO,
            time_interval,
        }
    }
    pub fn trigger(&mut self) -> bool {
        if self.cnt == 0 {
            self.cnt = self.cnt_interval;
            true
        } else {
            self.cnt -= 1;
            // check time interval
            let now = get_time_duration();
            if self.time_interval != Duration::ZERO
                && now.saturating_sub(self.last_time) >= self.time_interval
            {
                self.last_time = now;
                self.cnt = self.cnt_interval;
                return true;
            }
            false
        }
    }
}

lazy_static::lazy_static! {
    pub static ref CrossoverManager: SpinLock<BTreeMap<usize, IntermitCrossover>> = SpinLock::new(BTreeMap::new());
}

#[allow(unused)]
/// Execute a function every `interval` times
pub fn intermit(cnt_interval: Option<usize>, time_interval: Option<Duration>, f: impl FnOnce()) {
    assert!(
        cnt_interval.is_some() || time_interval.is_some(),
        "At least one of cnt_interval or time_interval must be specified"
    );

    let id = &f as *const _ as usize;
    let mut guard = CrossoverManager.lock();
    let time_interval = time_interval.unwrap_or(Duration::MAX);
    let cnt_interval = cnt_interval.unwrap_or(usize::MAX);
    let crossover = guard
        .entry(id)
        .or_insert(IntermitCrossover::new(cnt_interval, time_interval));
    if crossover.trigger() {
        f();
    }
}
