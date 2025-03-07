use async_task::ScheduleInfo;

use super::executor::RunnableTask;

pub trait Scheduler {
    fn default() -> Self;
    fn push(&mut self, runnable: RunnableTask, info: ScheduleInfo);
    fn pop(&mut self) -> Option<RunnableTask>;
    fn is_overload(&self) -> bool;
    fn is_underload(&self) -> bool;
}

// fixme: current sync scheme is incorrect, consider send ipi and flush cache
// fixme: use tlb-shoot-down to update mmap info (when multi-threading mmap)
