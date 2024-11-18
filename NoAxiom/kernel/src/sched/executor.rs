//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::collections::vec_deque::VecDeque;
use core::future::Future;

use async_task::{Builder, Runnable, ScheduleInfo, WithInfo};
use lazy_static::lazy_static;

use super::utask::TaskFuture;
use crate::sync::mutex::SpinMutex;

struct TaskInfo {
    prio: usize,
}

impl TaskInfo {
    pub const fn new(prio: usize) -> Self {
        Self { prio }
    }
}

pub(crate) struct Executor {
    queue: SpinMutex<VecDeque<Runnable<TaskInfo>>>,
}

impl Executor {
    pub const fn new() -> Self {
        Self {
            queue: SpinMutex::new(VecDeque::new()),
        }
    }
    fn push_back(&self, runnable: Runnable<TaskInfo>) {
        self.queue.lock().push_back(runnable);
    }
    fn push_front(&self, runnable: Runnable<TaskInfo>) {
        self.queue.lock().push_front(runnable);
    }
    fn pop_front(&self) -> Option<Runnable<TaskInfo>> {
        self.queue.lock().pop_front()
    }
}

lazy_static! {
    static ref EXECUTOR: Executor = Executor::new();
}

fn schedule(task: Runnable<TaskInfo>, info: ScheduleInfo) {
    let prio = task.metadata();
    if info.woken_while_running {
        EXECUTOR.push_front(task);
    } else {
        EXECUTOR.push_back(task);
    }
}

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    // let (task, handle) = async_task::spawn(future, WithInfo(schedule));
    let (task, handle) = Builder::new().metadata(TaskInfo::new(0)).spawn(
        move |info: &TaskInfo| TaskFuture::new(future, info.prio),
        WithInfo(schedule),
    );
    task.schedule();
    handle.detach();
}

/// Pop a task and run it
pub fn run() {
    // spin until find a valid task
    loop {
        if let Some(task) = EXECUTOR.pop_front() {
            task.metadata();
            task.run();
            break;
        }
    }
}
