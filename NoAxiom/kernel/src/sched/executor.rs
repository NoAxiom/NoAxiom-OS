//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::collections::{btree_map::BTreeMap, vec_deque::VecDeque};
use core::future::Future;

use async_task::{Builder, Runnable, ScheduleInfo, WithInfo};
use lazy_static::lazy_static;

use super::sched_entity::{SchedEntity, SchedVruntime};
use crate::sync::mutex::TicketMutex;

pub struct TaskScheduleInfo {
    sched_entity: SchedEntity,
}
impl TaskScheduleInfo {
    pub const fn new(sched_entity: SchedEntity) -> Self {
        Self { sched_entity }
    }
}

struct Executor {
    /// cfs tree: (prio, task)
    normal: BTreeMap<SchedVruntime, Runnable<TaskScheduleInfo>>,
    /// realtime / just-woken runnable queue
    urgent: VecDeque<Runnable<TaskScheduleInfo>>,
}
impl Executor {
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
    fn pop(&mut self) -> Option<Runnable<TaskScheduleInfo>> {
        if let Some(runnable) = self.urgent.pop_front() {
            Some(runnable)
        } else if let Some((_, runnable)) = self.normal.pop_first() {
            debug!(
                "poped from normal queue, vruntime: {}",
                runnable.metadata().sched_entity.inner().vruntime.0
            );
            for it in self.normal.iter() {
                debug!("normal queue: {:?}", it.1.metadata().sched_entity.inner());
            }
            Some(runnable)
        } else {
            None
        }
    }
}

// TODO: add muticore support
lazy_static! {
    static ref EXECUTOR: TicketMutex<Executor> = TicketMutex::new(Executor::new());
}

/// insert task into EXECUTOR when [`core::task::Waker::wake`] get called
fn schedule(runnable: Runnable<TaskScheduleInfo>, info: ScheduleInfo) {
    trace!(
        "[sched] schedule task, sched_entity: {:?}, woken_while_running: {}",
        runnable.metadata().sched_entity.inner(),
        info.woken_while_running
    );
    if info.woken_while_running {
        EXECUTOR.lock().push_normal(runnable);
    } else {
        EXECUTOR.lock().push_urgent(runnable);
    }
}

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(future: F, sched_entity: SchedEntity)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (runnable, handle) = Builder::new()
        .metadata(TaskScheduleInfo::new(sched_entity))
        .spawn(move |_: &TaskScheduleInfo| future, WithInfo(schedule));
    runnable.schedule();
    handle.detach();
}

/// Pop a task and run it
pub fn run() {
    // spin until find a valid task
    loop {
        let mut guard = EXECUTOR.lock();
        let runnable = guard.pop();
        drop(guard);
        if let Some(runnable) = runnable {
            runnable.run();
            break;
        }
    }
}
