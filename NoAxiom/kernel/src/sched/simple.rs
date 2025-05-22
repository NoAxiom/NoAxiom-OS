//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::collections::vec_deque::VecDeque;

use async_task::Runnable;
use ksync::{cell::SyncUnsafeCell, mutex::SpinLock};

use super::{
    sched_entity::SchedEntityWrapper,
    vsched::{Runtime, ScheduleOrder, Scheduler},
};

type Info = SchedEntityWrapper;

#[repr(align(64))]
pub struct SimpleScheduler {
    urgent: VecDeque<Runnable<Info>>,
    normal: VecDeque<Runnable<Info>>,
}

impl Scheduler<Info> for SimpleScheduler {
    fn new() -> Self {
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

struct FifoScheduler {
    queue: VecDeque<Runnable<Info>>,
}

impl Scheduler<Info> for FifoScheduler {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
    fn push_with_info(&mut self, runnable: Runnable<Info>, _: async_task::ScheduleInfo) {
        self.push_normal(runnable);
    }
    fn push_normal(&mut self, runnable: Runnable<Info>) {
        self.queue.push_back(runnable);
    }
    fn push_urgent(&mut self, runnable: Runnable<Info>) {
        self.queue.push_front(runnable);
    }
    fn pop(&mut self, _: ScheduleOrder) -> Option<Runnable<Info>> {
        self.queue.pop_front()
    }
}

type MultiSchedulerInnerImpl = SimpleScheduler;
pub struct MultiScheduler {
    current: SyncUnsafeCell<MultiSchedulerInnerImpl>,
    expire: SyncUnsafeCell<MultiSchedulerInnerImpl>,
}

impl MultiScheduler {
    fn switch_expire(&mut self) {
        core::mem::swap(&mut self.current, &mut self.expire);
    }
}

impl Scheduler<Info> for MultiScheduler {
    fn new() -> Self {
        Self {
            current: SyncUnsafeCell::new(MultiSchedulerInnerImpl::new()),
            expire: SyncUnsafeCell::new(MultiSchedulerInnerImpl::new()),
        }
    }
    fn push_with_info(&mut self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        self.expire.as_ref_mut().push_with_info(runnable, info);
    }
    fn push_normal(&mut self, runnable: Runnable<Info>) {
        self.expire.as_ref_mut().push_normal(runnable);
    }
    fn push_urgent(&mut self, runnable: Runnable<Info>) {
        self.expire.as_ref_mut().push_urgent(runnable);
    }
    fn pop(&mut self, order: ScheduleOrder) -> Option<Runnable<Info>> {
        let current = self.current.as_ref_mut();
        let res = current.pop(order);
        if let None = res.as_ref() {
            self.switch_expire();
        }
        res
    }
}

type SchedulerImpl = MultiScheduler;
pub struct SimpleRuntime {
    scheduler: SpinLock<SchedulerImpl>,
}

impl Runtime<SchedulerImpl, Info> for SimpleRuntime {
    fn new() -> Self {
        Self {
            scheduler: SpinLock::new(SchedulerImpl::new()),
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
