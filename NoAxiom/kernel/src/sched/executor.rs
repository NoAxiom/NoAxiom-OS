//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use core::future::Future;

use async_task::{Builder, Runnable, ScheduleInfo, WithInfo};
use lazy_static::lazy_static;

use super::utask::TaskFuture;
use crate::{config::sched::MLFQ_LEVELS, sync::mutex::SpinMutex};

struct TaskInfo {
    prio: usize,
}

impl TaskInfo {
    pub const fn new(prio: usize) -> Self {
        Self { prio }
    }
    pub fn level(&self) -> usize {
        self.prio
    }
}

pub(crate) struct Executor {
    queue: Vec<VecDeque<Runnable<TaskInfo>>>,
    begin_level: usize, // the level to start searching
}

impl Executor {
    pub fn new() -> Self {
        let mut vec = Vec::new();
        for _ in 0..MLFQ_LEVELS {
            vec.push(VecDeque::new());
        }
        Self {
            queue: vec,
            begin_level: 0,
        }
    }
    fn push_back(&mut self, level: usize, runnable: Runnable<TaskInfo>) {
        self.begin_level = self.begin_level.min(level);
        self.queue[level].push_back(runnable);
    }
    fn push_front(&mut self, level: usize, runnable: Runnable<TaskInfo>) {
        self.begin_level = self.begin_level.min(level);
        self.queue[level].push_front(runnable);
    }
    fn pop_front(&mut self) -> Option<Runnable<TaskInfo>> {
        for i in self.begin_level..MLFQ_LEVELS {
            let info = self.queue[i].pop_front();
            if info.is_some() {
                self.begin_level = i;
                return info;
            }
        }
        self.begin_level = MLFQ_LEVELS;
        None
    }
    #[inline(always)]
    fn update_begin_level(&mut self, level: usize) {
        self.begin_level = level;
    }
}

lazy_static! {
    static ref EXECUTOR: SpinMutex<Executor> = SpinMutex::new(Executor::new());
}

fn schedule(task: Runnable<TaskInfo>, info: ScheduleInfo) {
    let level = task.metadata().level();
    if info.woken_while_running {
        EXECUTOR.lock().push_front(level, task);
    } else {
        EXECUTOR.lock().push_back(level, task);
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
        if let Some(task) = EXECUTOR.lock().pop_front() {
            task.run();
            break;
        }
    }
}
