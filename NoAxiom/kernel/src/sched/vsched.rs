use async_task::{Runnable, ScheduleInfo};

#[derive(Clone, Copy, Debug)]
pub enum ScheduleOrder {
    NormalFirst,
    UrgentFirst,
}

pub trait Scheduler<R> {
    fn default() -> Self;
    fn push_with_info(&mut self, runnable: Runnable<R>, info: ScheduleInfo);
    fn push_normal(&mut self, runnable: Runnable<R>);
    fn push_urgent(&mut self, runnable: Runnable<R>);
    fn pop(&mut self, order: ScheduleOrder) -> Option<Runnable<R>>;
}

pub trait Runtime<T, R>
where
    T: Scheduler<R>,
{
    fn new() -> Self;
    fn run(&self);
    fn schedule(&self, runnable: Runnable<R>, info: ScheduleInfo);
}
