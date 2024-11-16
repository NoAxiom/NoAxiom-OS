//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::collections::vec_deque::VecDeque;
use core::future::Future;

use async_task::{Runnable, ScheduleInfo, WithInfo};
use lazy_static::lazy_static;

use crate::sync::mutex::SpinMutex;

pub(crate) struct Executor {
    queue: SpinMutex<VecDeque<Runnable>>,
}

impl Executor {
    pub const fn new() -> Self {
        Self {
            queue: SpinMutex::new(VecDeque::new()),
        }
    }
    fn push_back(&self, runnable: Runnable) {
        self.queue.lock().push_back(runnable);
    }
    fn push_front(&self, runnable: Runnable) {
        self.queue.lock().push_front(runnable);
    }
    fn pop_front(&self) -> Option<Runnable> {
        self.queue.lock().pop_front()
    }
}

lazy_static! {
    static ref EXECUTOR: Executor = Executor::new();
}

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
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

/// Pop a task and run it
pub fn run() {
    // spin until find a valid task
    loop {
        if let Some(task) = EXECUTOR.pop_front() {
            task.run();
            break;
        }
    }
}
