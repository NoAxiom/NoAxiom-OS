use alloc::sync::Arc;
use core::future::Future;

use async_task::{Runnable, ScheduleInfo};

use crate::task::Task;

pub trait Scheduler<R> {
    fn new() -> Self;
    fn push(&mut self, runnable: Runnable<R>, info: ScheduleInfo);
    fn pop(&mut self) -> Option<Runnable<R>>;
}

pub trait Runtime<R> {
    fn new() -> Self;
    fn run(&self);
    fn schedule(&self, runnable: Runnable<R>, info: ScheduleInfo);
    fn spawn<F>(self: &'static Self, future: F, task: Option<&Arc<Task>>)
    where
        F: Future<Output: Send + 'static> + Send + 'static;
}
