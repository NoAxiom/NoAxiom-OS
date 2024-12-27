//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use core::future::Future;

use async_task::{Builder, Runnable, ScheduleInfo, WithInfo};
use lazy_static::lazy_static;

use super::{
    sched_entity::SchedEntity,
    scheduler::{Scheduler, CFS},
};
use crate::sync::mutex::TicketMutex;

pub struct TaskScheduleInfo {
    pub sched_entity: SchedEntity,
}
impl TaskScheduleInfo {
    pub const fn new(sched_entity: SchedEntity) -> Self {
        Self { sched_entity }
    }
}

struct Runtime<T: Scheduler> {
    pub scheduler: TicketMutex<T>,
}

impl<T> Runtime<T>
where
    T: Scheduler,
{
    pub fn new() -> Self {
        Self {
            scheduler: TicketMutex::new(T::default()),
        }
    }
    pub fn schedule(&self, runnable: Runnable<TaskScheduleInfo>, info: ScheduleInfo) {
        self.scheduler.lock().push(runnable, info);
    }
    pub fn pop(&self) -> Option<Runnable<TaskScheduleInfo>> {
        self.scheduler.lock().pop()
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

/// Pop a task and run it
pub fn run() {
    // spin until find a valid task
    loop {
        let runnable = RUNTIME.pop();
        if let Some(runnable) = runnable {
            runnable.run();
            break;
        }
    }
}
