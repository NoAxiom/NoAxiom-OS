use alloc::collections::{btree_set::BTreeSet, vec_deque::VecDeque};
use core::cmp::Ordering;

use async_task::{Runnable, ScheduleInfo};

use super::{
    executor::TaskScheduleInfo,
    sched_entity::SchedVruntime,
    scheduler::{SchedLoadStats, Scheduler},
};

struct CfsTreeNode {
    pub vruntime: SchedVruntime,
    pub tid: usize,
    pub runnable: Runnable<TaskScheduleInfo>,
}
impl PartialEq for CfsTreeNode {
    fn eq(&self, other: &Self) -> bool {
        self.tid == other.tid
    }
}
impl Eq for CfsTreeNode {}
impl PartialOrd for CfsTreeNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let res = self.vruntime.partial_cmp(&other.vruntime);
        match res {
            Some(Ordering::Equal) => self.tid.partial_cmp(&other.tid),
            _ => res,
        }
    }
}
impl Ord for CfsTreeNode {
    fn cmp(&self, other: &Self) -> Ordering {
        let res = self.vruntime.cmp(&other.vruntime);
        match res {
            Ordering::Equal => self.tid.cmp(&other.tid),
            _ => res,
        }
    }
}

/// completely fair scheduler for single core
pub struct CFS {
    /// cfs tree
    normal: BTreeSet<CfsTreeNode>,
    /// realtime / just-woken runnable queue
    urgent: VecDeque<Runnable<TaskScheduleInfo>>,
    /// load: sum of load_weight of tasks in scheduler
    load: usize,
    /// counter of task
    task_count: usize,
}

impl CFS {
    pub const fn new() -> Self {
        Self {
            normal: BTreeSet::new(),
            urgent: VecDeque::new(),
            load: 0,
            task_count: 0,
        }
    }
    fn push_normal(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        self.load += runnable.metadata().sched_entity.get_load();
        self.task_count += 1;
        let vruntime = runnable.metadata().sched_entity.inner().vruntime;
        let tid = runnable.metadata().sched_entity.tid;
        self.normal.insert(CfsTreeNode {
            vruntime,
            tid,
            runnable,
        });
    }
    fn push_urgent(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        self.load += runnable.metadata().sched_entity.get_load();
        self.task_count += 1;
        self.urgent.push_back(runnable);
    }
}

impl Scheduler for CFS {
    /// default scheduler for init
    fn default() -> Self {
        Self::new()
    }

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
        trace!("pushed task, load: {:?}", self.load_stats());
    }

    /// pop a task from scheduler
    fn pop(&mut self) -> Option<Runnable<TaskScheduleInfo>> {
        let res = if let Some(runnable) = self.urgent.pop_front() {
            self.load -= runnable.metadata().sched_entity.get_load();
            self.task_count -= 1;
            Some(runnable)
        } else if let Some(node) = self.normal.pop_first() {
            let runnable = node.runnable;
            self.load -= runnable.metadata().sched_entity.get_load();
            self.task_count -= 1;
            Some(runnable)
        } else {
            None
        };
        if res.is_some() {
            trace!("normally poped task, load: {:?}", self.load_stats());
        }
        res
    }

    /// get load of scheduler
    /// return: (load, task_count)
    fn load_stats(&mut self) -> SchedLoadStats {
        SchedLoadStats {
            load: self.load,
            task_count: self.task_count,
        }
    }
}
