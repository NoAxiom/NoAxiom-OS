//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::collections::vec_deque::VecDeque;

use async_task::Runnable;
use ksync::mutex::SpinLock;

use super::{
    sched_info::SchedInfo,
    vsched::{Runtime, ScheduleOrder, Scheduler},
};

type Info = SchedInfo;

pub struct SimpleScheduler {
    queue: VecDeque<Runnable<Info>>,
}

impl Scheduler<Info> for SimpleScheduler {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
    fn push_with_info(&mut self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        match info.woken_while_running {
            true => self.queue.push_back(runnable),
            false => self.queue.push_front(runnable),
        }
    }
    fn push_normal(&mut self, runnable: Runnable<Info>) {
        self.queue.push_back(runnable);
    }
    fn push_urgent(&mut self, runnable: Runnable<Info>) {
        self.queue.push_front(runnable);
    }
    fn pop(&mut self, order: ScheduleOrder) -> Option<Runnable<Info>> {
        match order {
            ScheduleOrder::NormalFirst => self.queue.pop_back(),
            ScheduleOrder::UrgentFirst => self.queue.pop_front(),
        }
    }
}

pub struct SimpleRuntime<T>
where
    T: Scheduler<Info>,
{
    scheduler: SpinLock<T>,
}

impl<T> Runtime<T, Info> for SimpleRuntime<T>
where
    T: Scheduler<Info>,
{
    fn new() -> Self {
        Self {
            scheduler: SpinLock::new(T::default()),
        }
    }
    fn run(&self) {
        if let Some(runnable) = self.scheduler.lock().pop(ScheduleOrder::UrgentFirst) {
            runnable.run();
        }
    }
    fn schedule(&self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        self.scheduler.lock().push_with_info(runnable, info);
    }
}
