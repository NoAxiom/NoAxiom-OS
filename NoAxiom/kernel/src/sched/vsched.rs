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

pub trait MulticoreScheduler<R>
where
    Self: Scheduler<R>,
{
    fn sub_load(&mut self, load: usize);
    fn add_load(&mut self, load: usize);
    fn load(&self) -> usize;
    fn task_count(&self) -> usize;

    fn set_running(&mut self, is_running: bool);
    fn is_running(&self) -> bool;

    fn last_time(&self) -> usize;
    fn set_last_time(&mut self);
    fn set_time_limit(&mut self, limit: usize);
    fn is_timeup(&self) -> bool;
}

pub trait Runtime<T, R>
where
    T: Scheduler<R>,
{
    fn new() -> Self;
    fn run(&self);
    fn schedule(&self, runnable: Runnable<R>, info: ScheduleInfo);
}

pub trait MulticoreRuntime<T, R>
where
    Self: Runtime<T, R>,
    T: MulticoreScheduler<R>,
{
}
