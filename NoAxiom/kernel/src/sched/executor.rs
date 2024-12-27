//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use core::future::Future;

use array_init::array_init;
use async_task::{Builder, Runnable, ScheduleInfo, WithInfo};
use kernel_sync::SpinMutex;
use lazy_static::lazy_static;

use super::{
    sched_entity::SchedEntity,
    scheduler::{SchedulerLoadStats, Scheduler, CFS},
};
use crate::{config::arch::CPU_NUM, constant::sched::NICE_0_LOAD, cpu::get_hartid};

pub struct TaskScheduleInfo {
    pub sched_entity: SchedEntity,
}
impl TaskScheduleInfo {
    pub const fn new(sched_entity: SchedEntity) -> Self {
        Self { sched_entity }
    }
}

struct Runtime<T: Scheduler> {
    pub scheduler: [SpinMutex<T>; CPU_NUM],
}

impl<T> Runtime<T>
where
    T: Scheduler,
{
    pub fn new() -> Self {
        Self {
            scheduler: array_init(|_| SpinMutex::new(T::DEFAULT)),
        }
    }
    pub fn schedule(&self, runnable: Runnable<TaskScheduleInfo>, info: ScheduleInfo) {
        if info.woken_while_running {
            self.scheduler[get_hartid()].lock().push(runnable, info);
        } else {
            self.scheduler[min_load_hartid()]
                .lock()
                .push(runnable, info);
        }
    }
    pub fn pop(&self) -> Option<Runnable<TaskScheduleInfo>> {
        self.scheduler[get_hartid()].lock().pop()
    }
}

// TODO: add muticore support
lazy_static! {
    static ref RUNTIME: Runtime<CFS> = Runtime::new();
}

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(future: F, sched_entity: SchedEntity)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (runnable, handle) = Builder::new()
        .metadata(TaskScheduleInfo::new(sched_entity))
        .spawn(
            move |_: &TaskScheduleInfo| future,
            WithInfo(move |runnable, info| RUNTIME.schedule(runnable, info)),
        );
    runnable.schedule();
    handle.detach();
}

/// get the hartid who holds min load
fn min_load_hartid() -> usize {
    let mut min_load = usize::MAX;
    let mut min_hartid = 0;
    for (i, cfs) in RUNTIME.scheduler.iter().enumerate() {
        let load = cfs.lock().load_stats().load;
        if load < min_load {
            min_load = load;
            min_hartid = i;
        }
    }
    min_hartid
}

/// get the hartid who holds max load
/// return: (max_load, max_hartid)
fn max_load_hartid() -> (usize, SchedulerLoadStats) {
    let mut max_load = 0;
    let mut max_hartid = 0;
    let mut max_task_count = 0;
    let forbit_hart = get_hartid();
    for (i, cfs) in RUNTIME.scheduler.iter().enumerate() {
        if i != forbit_hart {
            let SchedulerLoadStats { load, task_count } = cfs.lock().load_stats();
            if load > max_load {
                max_load = load;
                max_hartid = i;
                max_task_count = task_count;
            }
        }
    }
    (
        max_hartid,
        SchedulerLoadStats {
            load: max_load,
            task_count: max_task_count,
        },
    )
}

/// load balance, fetch the task from max load hart
fn load_balance() -> Option<Runnable<TaskScheduleInfo>> {
    let current_hart = get_hartid();
    let (target_hart, load_info) = max_load_hartid();
    if target_hart != current_hart && load_info.task_count > 1 {
        let res = RUNTIME.scheduler[target_hart].lock().pop();
        if res.is_some() {
            warn!(
                "[load_balance] move task: hart {} -> hart {}",
                current_hart, target_hart,
            );
        }
        res
    } else {
        None
    }
}

/// Pop a task and run it
pub fn run() {
    // spin until find a valid task
    let runnable = RUNTIME.pop();
    if let Some(runnable) = runnable {
        runnable.run();
    } else if let Some(runnable) = load_balance() {
        runnable.run();
    }
}
