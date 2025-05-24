use async_task::{Runnable, ScheduleInfo};

pub trait Scheduler<R> {
    fn new() -> Self;
    fn push(&mut self, runnable: Runnable<R>, info: ScheduleInfo);
    fn pop(&mut self) -> Option<Runnable<R>>;
}

pub trait Runtime<T, R>
where
    T: Scheduler<R>,
{
    fn new() -> Self;
    fn run(&self);
    fn schedule(&self, runnable: Runnable<R>, info: ScheduleInfo);
}
