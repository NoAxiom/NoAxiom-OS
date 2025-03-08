use async_task::ScheduleInfo;

use super::executor::RunnableTask;

#[derive(Clone, Copy, Debug)]
pub enum ScheduleOrder {
    NormalFirst,
    UrgentFirst,
}

pub trait Scheduler {
    fn default() -> Self;
    fn push_with_info(&mut self, runnable: RunnableTask, info: ScheduleInfo);
    fn push_normal(&mut self, runnable: RunnableTask);
    fn push_urgent(&mut self, runnable: RunnableTask);
    fn pop(&mut self, order: ScheduleOrder) -> Option<RunnableTask>;
    fn is_overload(&self, all_load: usize) -> bool;
    fn is_underload(&self, all_load: usize) -> bool;
}
