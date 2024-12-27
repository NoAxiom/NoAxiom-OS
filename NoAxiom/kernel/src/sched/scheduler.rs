use alloc::collections::{btree_map::BTreeMap, vec_deque::VecDeque};

use async_task::{Runnable, ScheduleInfo};

use super::{executor::TaskScheduleInfo, sched_entity::SchedVruntime};
use crate::constant::sched::NICE_0_LOAD;

pub struct SchedulerLoadStats {
    pub load: usize,
    pub task_count: usize,
}

pub trait Scheduler {
    fn push(&mut self, runnable: Runnable<TaskScheduleInfo>, info: ScheduleInfo);
    fn pop(&mut self) -> Option<Runnable<TaskScheduleInfo>>;
    fn load_stats(&mut self) -> SchedulerLoadStats;
    const DEFAULT: Self;
}

/// completely fair scheduler for single core
pub struct CFS {
    /// cfs tree: (prio, task)
    normal: BTreeMap<SchedVruntime, Runnable<TaskScheduleInfo>>,
    /// realtime / just-woken runnable queue
    urgent: VecDeque<Runnable<TaskScheduleInfo>>,
    /// load: sum of load_weight of tasks in scheduler
    load: usize,
}

impl CFS {
    pub const fn new() -> Self {
        Self {
            normal: BTreeMap::new(),
            urgent: VecDeque::new(),
            load: 0,
        }
    }
    fn push_normal(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        self.load += runnable
            .metadata()
            .sched_entity
            .inner()
            .prio
            .to_load_weight();
        self.normal
            .insert(runnable.metadata().sched_entity.inner().vruntime, runnable);
    }
    fn push_urgent(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        self.load += NICE_0_LOAD;
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
            self.load -= NICE_0_LOAD;
            Some(runnable)
        } else if let Some((_, runnable)) = self.normal.pop_first() {
            // debug
            trace!(
                "poped from normal queue, vruntime: {}",
                runnable.metadata().sched_entity.inner().vruntime.0
            );
            for it in self.normal.iter() {
                trace!("normal queue: {:?}", it.1.metadata().sched_entity.inner());
            }
            // update load
            self.load -= runnable
                .metadata()
                .sched_entity
                .inner()
                .prio
                .to_load_weight();
            Some(runnable)
        } else {
            None
        }
    }
    /// get load of scheduler
    /// return: (load, task_count)
    fn load_stats(&mut self) -> SchedulerLoadStats {
        SchedulerLoadStats {
            load: self.load,
            task_count: self.normal.len() + self.urgent.len(),
        }
    }
    /// default scheduler for init
    const DEFAULT: Self = Self::new();
}
