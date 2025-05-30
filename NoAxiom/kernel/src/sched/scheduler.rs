use alloc::collections::vec_deque::VecDeque;

use async_task::Runnable;
use ksync::cell::SyncUnsafeCell;

use super::{
    sched_entity::{SchedMetadata, SchedPrio},
    vsched::Scheduler,
};

pub(super) type Info = SchedMetadata;

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

pub struct DualPrioScheduler {
    normal: FifoScheduler,
    idle: FifoScheduler,
}

impl Scheduler<Info> for DualPrioScheduler {
    fn new() -> Self {
        Self {
            normal: FifoScheduler::new(),
            idle: FifoScheduler::new(),
        }
    }
    fn push(&mut self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        match info.woken_while_running {
            true => self.idle.push(runnable, info),
            false => self.normal.push(runnable, info),
        }
    }
    fn pop(&mut self) -> Option<Runnable<Info>> {
        if let Some(runnable) = self.normal.pop() {
            return Some(runnable);
        }
        self.idle.pop()
    }
}

type ExpiredSchedulerInnerImpl = DualPrioScheduler;
pub struct ExpiredScheduler {
    current: SyncUnsafeCell<ExpiredSchedulerInnerImpl>,
    expire: SyncUnsafeCell<ExpiredSchedulerInnerImpl>,
}

impl ExpiredScheduler {
    fn switch_expire(&mut self) {
        core::mem::swap(&mut self.current, &mut self.expire);
    }
}

impl Scheduler<Info> for ExpiredScheduler {
    fn new() -> Self {
        Self {
            current: SyncUnsafeCell::new(ExpiredSchedulerInnerImpl::new()),
            expire: SyncUnsafeCell::new(ExpiredSchedulerInnerImpl::new()),
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

type RealTimeSchedulerImpl = FifoScheduler;
type NormalSchedulerImpl = ExpiredScheduler;
#[repr(align(64))]
pub struct MultiLevelScheduler {
    realtime: RealTimeSchedulerImpl,
    normal: NormalSchedulerImpl,
}

impl Scheduler<Info> for MultiLevelScheduler {
    fn new() -> Self {
        Self {
            realtime: RealTimeSchedulerImpl::new(),
            normal: NormalSchedulerImpl::new(),
        }
    }
    fn push(&mut self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        let entity = runnable.metadata().sched_entity();
        if let Some(entity) = entity {
            match entity.sched_prio {
                SchedPrio::RealTime(_) => self.realtime.push(runnable, info),
                _ => self.normal.push(runnable, info),
            }
        } else {
            self.realtime.push(runnable, info);
        }
    }
    fn pop(&mut self) -> Option<Runnable<Info>> {
        if let Some(runnable) = self.realtime.pop() {
            return Some(runnable);
        }
        self.normal.pop()
    }
}
