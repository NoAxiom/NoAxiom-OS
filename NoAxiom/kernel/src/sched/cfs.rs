use alloc::collections::{btree_set::BTreeSet, vec_deque::VecDeque};

use async_task::{Runnable, ScheduleInfo};

use super::{
    executor::TaskScheduleInfo,
    sched_entity::SchedVruntime,
    scheduler::{SchedLoadStats, Scheduler},
};

struct CfsTreeNode {
    pub vruntime: SchedVruntime,
    pub runnable: Runnable<TaskScheduleInfo>,
}
impl PartialEq for CfsTreeNode {
    fn eq(&self, _others: &Self) -> bool {
        // always return false to convert it into multiset
        false
    }
}
impl Eq for CfsTreeNode {}
impl PartialOrd for CfsTreeNode {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        let res = self.vruntime.partial_cmp(&other.vruntime);
        match res {
            Some(core::cmp::Ordering::Equal) => Some(core::cmp::Ordering::Less),
            _ => res,
        }
    }
}
impl Ord for CfsTreeNode {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let res = self.vruntime.cmp(&other.vruntime);
        match res {
            // never return equal to convert it into multiset
            core::cmp::Ordering::Equal => core::cmp::Ordering::Less,
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
        // hack: use multiset to avoid vruntime conflict
        // let vruntime = SchedVruntime::new(0);
        self.normal.insert(CfsTreeNode { vruntime, runnable });
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
