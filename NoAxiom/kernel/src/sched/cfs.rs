use alloc::collections::{btree_set::BTreeSet, vec_deque::VecDeque};
use core::cmp::Ordering;

use async_task::ScheduleInfo;

use super::{
    executor::{RunnableTask, RUNTIME},
    sched_entity::SchedVruntime,
    scheduler::Scheduler,
};
use crate::{
    config::{arch::CPU_NUM, sched::LOAD_BALANCE_LIMIT},
    constant::sched::NICE_0_LOAD,
};

struct CfsTreeNode {
    pub vruntime: SchedVruntime,
    pub tid: usize,
    pub runnable: RunnableTask,
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
    urgent: VecDeque<RunnableTask>,
    /// load: sum of load_weight of tasks in scheduler
    load: usize,
    /// counter of task
    task_count: usize,
}

impl CFS {
    fn sub_load(&mut self, load: usize) {
        RUNTIME.sub_load(load);
        self.load -= load;
        self.task_count -= 1;
    }
    fn add_load(&mut self, load: usize) {
        RUNTIME.add_load(load);
        self.load += load;
        self.task_count += 1;
    }
    pub fn push_normal(&mut self, runnable: RunnableTask) {
        self.add_load(runnable.metadata().sched_entity.get_load());
        let vruntime = runnable.metadata().sched_entity.inner().vruntime;
        let tid = runnable.metadata().sched_entity.tid;
        self.normal.insert(CfsTreeNode {
            vruntime,
            tid,
            runnable,
        });
    }
    fn push_urgent(&mut self, runnable: RunnableTask) {
        self.add_load(runnable.metadata().sched_entity.get_load());
        self.urgent.push_back(runnable);
    }
}

impl Scheduler for CFS {
    /// default scheduler for init
    fn default() -> Self {
        Self {
            normal: BTreeSet::new(),
            urgent: VecDeque::new(),
            load: 0,
            task_count: 0,
        }
    }

    /// insert task into scheduler when [`core::task::Waker::wake`] get called
    fn push(&mut self, runnable: RunnableTask, info: ScheduleInfo) {
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
    fn pop(&mut self) -> Option<RunnableTask> {
        let res = if let Some(runnable) = self.urgent.pop_front() {
            self.sub_load(runnable.metadata().sched_entity.get_load());
            Some(runnable)
        } else if let Some(node) = self.normal.pop_first() {
            let runnable = node.runnable;
            self.sub_load(runnable.metadata().sched_entity.get_load());
            Some(runnable)
        } else {
            None
        };
        res
    }

    /// check if scheduler is overloaded
    fn is_overload(&self) -> bool {
        let all_load = RUNTIME.get_load();
        let ave = all_load / CPU_NUM;
        // if self.task_count > 1 {
        //     warn!(
        //         "overload: load: {}, task_count: {}, ave: {}, res: {}",
        //         self.load,
        //         self.task_count,
        //         ave,
        //         self.load > ave + ave / LOAD_BALANCE_LIMIT && self.task_count > 1
        //     );
        // }
        self.load > ave + ave / LOAD_BALANCE_LIMIT && self.task_count > 1
    }

    /// check if scheduler is underloaded
    fn is_underload(&self) -> bool {
        let all_load = RUNTIME.get_load();
        let ave = all_load / CPU_NUM;
        self.load + ave / LOAD_BALANCE_LIMIT < ave && all_load > NICE_0_LOAD
    }
}
