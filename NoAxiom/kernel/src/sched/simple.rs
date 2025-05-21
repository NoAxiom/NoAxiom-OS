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

#[repr(align(64))]
pub struct SimpleScheduler {
    urgent: VecDeque<Runnable<Info>>,
    normal: VecDeque<Runnable<Info>>,
}

impl Scheduler<Info> for SimpleScheduler {
    fn default() -> Self {
        Self {
            urgent: VecDeque::new(),
            normal: VecDeque::new(),
        }
    }
    fn push_with_info(&mut self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        match info.woken_while_running {
            true => self.push_normal(runnable),
            false => self.push_urgent(runnable),
        }
    }
    fn push_normal(&mut self, runnable: Runnable<Info>) {
        self.normal.push_back(runnable);
    }
    fn push_urgent(&mut self, runnable: Runnable<Info>) {
        self.urgent.push_back(runnable);
    }
    fn pop(&mut self, _: ScheduleOrder) -> Option<Runnable<Info>> {
        if let Some(runnable) = self.urgent.pop_front() {
            return Some(runnable);
        }
        if let Some(runnable) = self.normal.pop_front() {
            return Some(runnable);
        }
        None
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
        let runnable = self.scheduler.lock().pop(ScheduleOrder::UrgentFirst);
        if let Some(runnable) = runnable {
            runnable.run();
        }
    }
    fn schedule(&self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        self.scheduler.lock().push_with_info(runnable, info);
    }
}
