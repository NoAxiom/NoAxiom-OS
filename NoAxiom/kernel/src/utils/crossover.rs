use alloc::collections::btree_map::BTreeMap;
use core::sync::atomic::AtomicUsize;

use ksync::mutex::SpinLock;

#[allow(unused)]
/// Frequency division
pub struct Crossover {
    cnt: AtomicUsize,
    interval: usize,
}

#[allow(unused)]
impl Crossover {
    pub fn new(interval: usize) -> Self {
        Crossover {
            cnt: AtomicUsize::new(interval),
            interval,
        }
    }
    pub fn trigger(&self) -> bool {
        if self.cnt.load(core::sync::atomic::Ordering::SeqCst) == 0 {
            self.cnt
                .store(self.interval, core::sync::atomic::Ordering::SeqCst);
            true
        } else {
            self.cnt.fetch_sub(1, core::sync::atomic::Ordering::SeqCst);
            false
        }
    }
}

lazy_static::lazy_static! {
    pub static ref CrossoverManager: SpinLock<BTreeMap<usize, Crossover>> = SpinLock::new(BTreeMap::new());
}
