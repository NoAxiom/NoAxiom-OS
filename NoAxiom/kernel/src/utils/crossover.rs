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

#[allow(unused)]
/// Execute a function every `interval` times
pub fn intermit(f: impl FnOnce()) {
    let interval = 89102;
    let id = &f as *const _ as usize;
    let mut guard = CrossoverManager.lock();
    let crossover = guard.entry(id).or_insert(Crossover::new(interval));
    if crossover.trigger() {
        f();
    }
}