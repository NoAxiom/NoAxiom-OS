use alloc::collections::{btree_set::BTreeSet, vec_deque::VecDeque};
use core::cmp::Ordering;

use async_task::{Runnable, ScheduleInfo};

use super::{
    sched_entity::SchedVruntime,
    sched_info::SchedInfo,
    vsched::{MulticoreScheduler, ScheduleOrder, Scheduler},
};
use crate::time::{gettime::get_time, time_slice::get_load_balance_ticks};

struct CfsTreeNode<R> {
    pub vruntime: SchedVruntime,
    pub tid: usize,
    pub runnable: Runnable<R>,
}
impl<R> PartialEq for CfsTreeNode<R> {
    fn eq(&self, other: &Self) -> bool {
        self.tid == other.tid
    }
}
impl<R> Eq for CfsTreeNode<R> {}
impl<R> PartialOrd for CfsTreeNode<R> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let res = self.vruntime.partial_cmp(&other.vruntime);
        match res {
            Some(Ordering::Equal) => self.tid.partial_cmp(&other.tid),
            _ => res,
        }
    }
}
impl<R> Ord for CfsTreeNode<R> {
    fn cmp(&self, other: &Self) -> Ordering {
        let res = self.vruntime.cmp(&other.vruntime);
        match res {
            Ordering::Equal => self.tid.cmp(&other.tid),
            _ => res,
        }
    }
}

/// completely fair scheduler for single core
#[repr(align(64))]
pub struct CFS<R> {
    /// cfs tree
    normal: BTreeSet<CfsTreeNode<R>>,
    /// realtime / just-woken runnable queue
    urgent: VecDeque<Runnable<R>>,
    /// load: sum of load_weight of tasks in scheduler
    load: usize,
    /// counter of task
    task_count: usize,
    /// last load balance time (tick)
    last_time: usize,
    /// load balance time limit
    time_limit: usize,
    /// is running
    is_running: bool,
}

impl<R> MulticoreScheduler<R> for CFS<R>
where
    Self: Scheduler<R>,
{
    /// sub local load
    fn sub_load(&mut self, load: usize) {
        self.load -= load;
        self.task_count -= 1;
    }
    /// add local load
    fn add_load(&mut self, load: usize) {
        self.load += load;
        self.task_count += 1;
    }
    /// fetch local load value
    fn load(&self) -> usize {
        self.load
    }
    /// fetch local task count
    fn task_count(&self) -> usize {
        self.task_count
    }
    /// fetch last load balance time
    fn last_time(&self) -> usize {
        self.last_time
    }
    /// is time up for load balance
    fn is_timeup(&self) -> bool {
        get_time() as isize - self.last_time as isize > self.time_limit as isize
    }
    /// set last load balance time
    fn set_last_time(&mut self) {
        self.time_limit = get_load_balance_ticks();
        self.last_time = get_time();
    }
    /// set time limit for load balance
    fn set_time_limit(&mut self, limit: usize) {
        self.time_limit = limit;
    }
    /// is running a task
    fn is_running(&self) -> bool {
        self.is_running
    }
    /// set running status
    fn set_running(&mut self, is_running: bool) {
        self.is_running = is_running;
    }
}

impl Scheduler<SchedInfo> for CFS<SchedInfo> {
    /// default scheduler for init
    fn default() -> Self {
        Self {
            normal: BTreeSet::new(),
            urgent: VecDeque::new(),
            load: 0,
            task_count: 0,
            last_time: 0,
            time_limit: get_load_balance_ticks(),
            is_running: false,
        }
    }

    /// insert task into scheduler when [`core::task::Waker::wake`] get called
    fn push_with_info(&mut self, runnable: Runnable<SchedInfo>, info: ScheduleInfo) {
        if info.woken_while_running {
            self.push_normal(runnable);
        } else {
            self.push_urgent(runnable);
        }
    }

    /// push a task to the normal queue, aka cfs tree
    fn push_normal(&mut self, runnable: Runnable<SchedInfo>) {
        self.add_load(runnable.metadata().sched_entity.get_load());
        let vruntime = runnable.metadata().sched_entity.inner().vruntime;
        let tid = runnable.metadata().sched_entity.tid;
        self.normal.insert(CfsTreeNode {
            vruntime,
            tid,
            runnable,
        });
    }

    /// push a task to the urgent queue
    fn push_urgent(&mut self, runnable: Runnable<SchedInfo>) {
        self.add_load(runnable.metadata().sched_entity.get_load());
        self.urgent.push_back(runnable);
    }

    /// pop a task from scheduler
    fn pop(&mut self, order: ScheduleOrder) -> Option<Runnable<SchedInfo>> {
        match order {
            ScheduleOrder::UrgentFirst => {
                if let Some(runnable) = self.urgent.pop_front() {
                    trace!("[sched_pop] pop urgent task (urgent first)");
                    self.sub_load(runnable.metadata().sched_entity.get_load());
                    Some(runnable)
                } else if let Some(node) = self.normal.pop_first() {
                    trace!("[sched_pop] pop normal task (urgent first)");
                    let runnable = node.runnable;
                    self.sub_load(runnable.metadata().sched_entity.get_load());
                    Some(runnable)
                } else {
                    None
                }
            }
            ScheduleOrder::NormalFirst => {
                if let Some(node) = self.normal.pop_first() {
                    trace!("[sched_pop] pop normal task (normal first)");
                    let runnable = node.runnable;
                    self.sub_load(runnable.metadata().sched_entity.get_load());
                    Some(runnable)
                } else if let Some(runnable) = self.urgent.pop_front() {
                    trace!("[sched_pop] pop urgent task (normal first)");
                    self.sub_load(runnable.metadata().sched_entity.get_load());
                    Some(runnable)
                } else {
                    None
                }
            }
        }
    }
}
