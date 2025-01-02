//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::sync::Arc;
use core::future::Future;

use array_init::array_init;
use async_task::{Builder, Runnable, ScheduleInfo, WithInfo};
use ksync::mutex::SpinLock;
use lazy_static::lazy_static;

use super::{
    cfs::CFS,
    sched_entity::{SchedEntity, SchedTaskInfo},
    scheduler::{SchedLoadStats, Scheduler},
};
use crate::{config::arch::CPU_NUM, cpu::get_hartid};

pub struct TaskScheduleInfo {
    pub sched_entity: SchedEntity,
    pub task_info: Option<SchedTaskInfo>,
}
impl TaskScheduleInfo {
    pub fn new(sched_entity: SchedEntity, task_info: Option<SchedTaskInfo>) -> Self {
        Self {
            sched_entity,
            task_info,
        }
    }
}

struct Runtime<T: Scheduler> {
    pub scheduler: [SpinLock<T>; CPU_NUM],
}

impl<T> Runtime<T>
where
    T: Scheduler,
{
    pub fn new() -> Self {
        Self {
            scheduler: array_init(|_| SpinLock::new(T::default())),
        }
    }
    pub fn schedule(&self, runnable: Runnable<TaskScheduleInfo>, info: ScheduleInfo) {
        #[cfg(feature = "multicore")]
        {
            // normally we schedule task in local queue
            // but if the task isn't shared by other thread
            // then we can freely move it to other hart
            // for ktask, spawn it in local hart as well
            let mut hart = get_hartid();
            if !info.woken_while_running {
                if let Some(info) = runnable.metadata().task_info.as_ref() {
                    if Arc::strong_count(&info.task.memory_set) <= 1 {
                        hart = min_load_hartid();
                    }
                }
            }
            self.scheduler[hart].lock().push(runnable, info);
        }
        #[cfg(not(feature = "multicore"))]
        {
            self.scheduler[get_hartid()].lock().push(runnable, info);
        }
    }
    pub fn pop_current(&self) -> Option<Runnable<TaskScheduleInfo>> {
        self.scheduler[get_hartid()].lock().pop()
    }
}

// TODO: add muticore support
lazy_static! {
    static ref RUNTIME: Runtime<CFS> = Runtime::new();
}

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(future: F, sched_entity: SchedEntity, task_info: Option<SchedTaskInfo>)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (runnable, handle) = Builder::new()
        .metadata(TaskScheduleInfo::new(sched_entity, task_info))
        .spawn(
            move |_: &TaskScheduleInfo| future,
            WithInfo(move |runnable, info| RUNTIME.schedule(runnable, info)),
        );
    runnable.schedule();
    handle.detach();
}

// TODO: don't calc it every time, instead calc it when push/pop happens
/// get the hartid who holds min load
fn min_load_hartid() -> usize {
    #[cfg(not(feature = "multicore"))]
    {
        trace!("min_load_hartid: single core, return current hartid");
        return get_hartid();
    }

    let mut min_load = usize::MAX;
    let mut min_hartid = get_hartid();
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
fn max_load_hartid() -> Option<usize> {
    let mut max_load = 0;
    let mut max_hartid = 0;
    let mut flag = false;
    let forbit_hart = get_hartid();
    for (i, cfs) in RUNTIME.scheduler.iter().enumerate() {
        if i != forbit_hart {
            let SchedLoadStats { load, task_count } = cfs.lock().load_stats();
            if load > max_load && task_count > 1 {
                max_load = load;
                max_hartid = i;
                flag = true;
            }
        }
    }
    match flag {
        true => Some(max_hartid),
        false => None,
    }
}

/// load balance, fetch the task from max load hart
fn load_balance() -> Option<Runnable<TaskScheduleInfo>> {
    let current_hart = get_hartid();
    if let Some(from_hart) = max_load_hartid() {
        let res = RUNTIME.scheduler[from_hart].lock().steal();
        if res.is_some() {
            warn!(
                "[load_balance] move task: hart {} -> hart: {}",
                from_hart,    // RUNTIME.scheduler[from_hart].lock().load_stats().load,
                current_hart, // RUNTIME.scheduler[current_hart].lock().load_stats().load
            );
        }
        return res;
    }
    None
}

/// Pop a task and run it
pub fn run() {
    // spin until find a valid task
    let runnable = RUNTIME.pop_current();
    if let Some(runnable) = runnable {
        runnable.run();
    } else {
        // TODO: 使用请求模式而不是抢占模式进行负载均衡
        #[cfg(feature = "multicore")]
        if let Some(runnable) = load_balance() {
            runnable.run();
        }
    }
}
