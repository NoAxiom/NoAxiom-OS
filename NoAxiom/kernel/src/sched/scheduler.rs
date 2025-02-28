use async_task::{Runnable, ScheduleInfo};

use super::executor::TaskScheduleInfo;

#[derive(Debug, Clone, Copy)]
pub struct SchedLoadStats {
    pub load: usize,
    pub task_count: usize,
}

pub trait Scheduler {
    fn default() -> Self;
    fn push(&mut self, runnable: Runnable<TaskScheduleInfo>, info: ScheduleInfo);
    fn pop(&mut self) -> Option<Runnable<TaskScheduleInfo>>;
    fn load_stats(&mut self) -> SchedLoadStats;
    fn be_stolen(&mut self) -> Option<Runnable<TaskScheduleInfo>> {
        let res = self.pop();
        // todo
        res
    }
}

// fixme: current sync scheme is incorrect, consider send ipi and flush cache
// fixme: use tlb-shoot-down to update mmap info (when multi-threading mmap)
