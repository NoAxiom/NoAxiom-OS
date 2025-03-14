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
    R: MulticoreSchedInfo,
{
    fn sub_load(&mut self, load: usize);
    fn add_load(&mut self, load: usize);
    fn is_overload(&self, all_load: usize) -> bool;
    fn is_underload(&self, all_load: usize) -> bool;
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
    R: MulticoreSchedInfo,
{
    fn add_load(&self, load: usize);
    fn sub_load(&self, load: usize);
}

pub trait MulticoreSchedInfo {
    fn set_hartid(&self, hartid: usize);
    fn hartid(&self) -> usize;
}
