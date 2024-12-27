use alloc::collections::{btree_map::BTreeMap, vec_deque::VecDeque};

use async_task::{Runnable, ScheduleInfo};

use super::{executor::TaskScheduleInfo, sched_entity::SchedVruntime};

pub trait Scheduler: Default {
    fn push(&mut self, runnable: Runnable<TaskScheduleInfo>, info: ScheduleInfo);
    fn pop(&mut self) -> Option<Runnable<TaskScheduleInfo>>;
}

pub struct CFS {
    /// cfs tree: (prio, task)
    normal: BTreeMap<SchedVruntime, Runnable<TaskScheduleInfo>>,
    /// realtime / just-woken runnable queue
    urgent: VecDeque<Runnable<TaskScheduleInfo>>,
}

impl CFS {
    pub fn new() -> Self {
        Self {
            normal: BTreeMap::new(),
            urgent: VecDeque::new(),
        }
    }
    fn push_normal(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        self.normal
            .insert(runnable.metadata().sched_entity.inner().vruntime, runnable);
    }
    fn push_urgent(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        self.urgent.push_back(runnable);
    }
}

impl Scheduler for CFS {
    /// insert task into scheduler when [`core::task::Waker::wake`] get called
    fn push(&mut self, runnable: Runnable<TaskScheduleInfo>, info: ScheduleInfo) {
        trace!(
            "[sched] schedule task, sched_entity: {:?}, woken_while_running: {}",
            runnable.metadata().sched_entity.inner(),
            info.woken_while_running
        );
        if info.woken_while_running {
            self.push_normal(runnable);
        } else {
            self.push_urgent(runnable);
        }
    }
    /// pop a task from scheduler
    fn pop(&mut self) -> Option<Runnable<TaskScheduleInfo>> {
        if let Some(runnable) = self.urgent.pop_front() {
            Some(runnable)
        } else if let Some((_, runnable)) = self.normal.pop_first() {
            trace!(
                "poped from normal queue, vruntime: {}",
                runnable.metadata().sched_entity.inner().vruntime.0
            );
            for it in self.normal.iter() {
                trace!("normal queue: {:?}", it.1.metadata().sched_entity.inner());
            }
            Some(runnable)
        } else {
            None
        }
    }
}

impl Default for CFS {
    fn default() -> Self {
        Self::new()
    }
}
