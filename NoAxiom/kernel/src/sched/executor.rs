//! executor

use alloc::collections::vec_deque::VecDeque;
use core::future::Future;

use async_task::{Runnable, ScheduleInfo, WithInfo};
use ksync::mutex::SpinMutex;
use lazy_static::lazy_static;

struct Executor {
    scheduler: SpinMutex<VecDeque<Runnable>>,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            scheduler: SpinMutex::new(VecDeque::new()),
        }
    }
    pub fn push_back(&self, runnable: Runnable) {
        self.scheduler.lock().push_back(runnable);
    }
    pub fn push_front(&self, runnable: Runnable) {
        self.scheduler.lock().push_front(runnable);
    }
    pub fn pop_front(&self) -> Option<Runnable> {
        self.scheduler.lock().pop_front()
    }
}

lazy_static! {
    static ref EXECUTOR: Executor = Executor::new();
}

/// Add a task into task queue
pub fn spawn<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    // TODO: add MLFQ scheduler here
    let (task, handle) = async_task::spawn(
        future,
        WithInfo(move |task: Runnable, info: ScheduleInfo| {
            if info.woken_while_running {
                EXECUTOR.push_back(task);
            } else {
                EXECUTOR.push_front(task);
            }
        }),
    );
    task.schedule();
    handle.detach();
}

pub fn run() {
    if let Some(task) = EXECUTOR.pop_front() {
        let runnable_task: Runnable = task;
        runnable_task.run();
    }
}
