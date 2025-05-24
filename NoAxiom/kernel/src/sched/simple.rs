//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::collections::vec_deque::VecDeque;

use async_task::Runnable;
use ksync::{cell::SyncUnsafeCell, mutex::SpinLock};

use super::{
    sched_entity::{SchedEntityWrapper, SchedPrio},
    vsched::{Runtime, Scheduler},
};

type Info = SchedEntityWrapper;

#[repr(align(64))]
pub struct SimpleScheduler {
    urgent: VecDeque<Runnable<Info>>,
    normal: VecDeque<Runnable<Info>>,
}

impl SimpleScheduler {
    fn push_normal(&mut self, runnable: Runnable<Info>) {
        self.normal.push_back(runnable);
    }
    fn push_urgent(&mut self, runnable: Runnable<Info>) {
        self.urgent.push_back(runnable);
    }
}

impl Scheduler<Info> for SimpleScheduler {
    fn new() -> Self {
        Self {
            urgent: VecDeque::new(),
            normal: VecDeque::new(),
        }
    }
    fn push(&mut self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        match info.woken_while_running {
            true => self.push_normal(runnable),
            false => self.push_urgent(runnable),
        }
    }
    fn pop(&mut self) -> Option<Runnable<Info>> {
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
    fn push(&mut self, runnable: Runnable<Info>, _: async_task::ScheduleInfo) {
        self.queue.push_back(runnable);
    }
    fn pop(&mut self) -> Option<Runnable<Info>> {
        self.queue.pop_front()
    }
}

pub struct MultiLevelScheduler {
    realtime: FifoScheduler,
    normal: ExpiredScheduler,
    idle: ExpiredScheduler,
}

impl Scheduler<Info> for MultiLevelScheduler {
    fn new() -> Self {
        Self {
            realtime: FifoScheduler::new(),
            normal: ExpiredScheduler::new(),
            idle: ExpiredScheduler::new(),
        }
    }
    fn push(&mut self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        let entity = runnable.metadata().sched_entity();
        if let Some(entity) = entity {
            match entity.sched_prio {
                SchedPrio::RealTime(_) => {
                    // println!(
                    //     "push realtime runnable, time_stat = {:?}, realtime size = {}",
                    //     runnable
                    //         .metadata()
                    //         .sched_entity()
                    //         .unwrap()
                    //         .time_stat
                    //         .stime(),
                    //     self.realtime.queue.len(),
                    // );
                    self.realtime.push(runnable, info)
                }
                SchedPrio::Normal => self.normal.push(runnable, info),
                SchedPrio::IdlePrio => self.idle.push(runnable, info),
            }
        } else {
            self.normal.push(runnable, info);
        }
    }
    fn pop(&mut self) -> Option<Runnable<Info>> {
        if let Some(runnable) = self.realtime.pop() {
            // println!(
            //     "pop realtime runnable, time_stat = {:?}, realtime size = {}",
            //     runnable
            //         .metadata()
            //         .sched_entity()
            //         .unwrap()
            //         .time_stat
            //         .stime(),
            //     self.realtime.queue.len(),
            // );
            return Some(runnable);
        }
        if let Some(runnable) = self.normal.pop() {
            return Some(runnable);
        }
        self.idle.pop()
    }
}

type MultiSchedulerInnerImpl = FifoScheduler;
pub struct ExpiredScheduler {
    current: SyncUnsafeCell<MultiSchedulerInnerImpl>,
    expire: SyncUnsafeCell<MultiSchedulerInnerImpl>,
}

impl ExpiredScheduler {
    fn switch_expire(&mut self) {
        core::mem::swap(&mut self.current, &mut self.expire);
    }
}

impl Scheduler<Info> for ExpiredScheduler {
    fn new() -> Self {
        Self {
            current: SyncUnsafeCell::new(MultiSchedulerInnerImpl::new()),
            expire: SyncUnsafeCell::new(MultiSchedulerInnerImpl::new()),
        }
    }
    fn push(&mut self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        self.expire.as_ref_mut().push(runnable, info);
    }
    fn pop(&mut self) -> Option<Runnable<Info>> {
        let current = self.current.as_ref_mut();
        let res = current.pop();
        if let None = res.as_ref() {
            self.switch_expire();
            self.current.as_ref_mut().pop()
        } else {
            res
        }
    }
}

type SchedulerImpl = MultiLevelScheduler;
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
        let runnable = self.scheduler.lock().pop();
        if let Some(runnable) = runnable {
            runnable.run();
        }
    }
    fn schedule(&self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        self.scheduler.lock().push(runnable, info);
    }
}
