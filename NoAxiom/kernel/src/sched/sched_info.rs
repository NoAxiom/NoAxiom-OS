use alloc::sync::Weak;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::{sched_entity::SchedEntity, vsched::MulticoreSchedInfo};
use crate::task::Task;

pub struct SchedInfo {
    pub sched_entity: SchedEntity,
    /// the hartid that the task should be running on
    pub hartid: AtomicUsize,
    pub task: Option<Weak<Task>>,
}
impl SchedInfo {
    pub fn new(sched_entity: SchedEntity, hartid: usize, task: Option<Weak<Task>>) -> Self {
        Self {
            sched_entity,
            hartid: AtomicUsize::new(hartid),
            task,
        }
    }
}

impl MulticoreSchedInfo for SchedInfo {
    fn set_hartid(&self, hartid: usize) {
        self.hartid.store(hartid, Ordering::Release);
    }
    fn hartid(&self) -> usize {
        self.hartid.load(Ordering::Acquire)
    }
}
