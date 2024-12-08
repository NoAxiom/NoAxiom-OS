//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::{collections::vec_deque::VecDeque, sync::Arc, vec::Vec};
use core::future::Future;

use async_task::{Builder, Runnable, ScheduleInfo, WithInfo};
use lazy_static::lazy_static;

use crate::{
    config::sched::MLFQ_LEVELS,
    sync::{cell::SyncUnsafeCell, mutex::SpinMutex},
};

pub struct TaskScheduleInfo {
    prio: Arc<SyncUnsafeCell<isize>>,
}
impl TaskScheduleInfo {
    pub const fn new(prio: Arc<SyncUnsafeCell<isize>>) -> Self {
        Self { prio }
    }
    pub fn prio(&self) -> &isize {
        unsafe { &(*self.prio.get()) }
    }
}

struct Executor {
    queue: Vec<VecDeque<Runnable<TaskScheduleInfo>>>,
}
impl Executor {
    pub fn new() -> Self {
        let mut vec = Vec::new();
        for _ in 0..MLFQ_LEVELS {
            vec.push(VecDeque::new());
        }
        Self { queue: vec }
    }
    fn push_back(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        let level = runnable.metadata().prio();
        trace!("[sched] push task to back, prio: {}", level);
        // self.queue[level as usize].push_back(runnable);
        self.queue[0].push_back(runnable);
    }
    fn push_front(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        let level = runnable.metadata().prio();
        trace!("[sched] push task to front, prio: {}", level);
        // self.queue[level as usize].push_front(runnable);
        self.queue[0].push_front(runnable);
    }
    fn pop_front(&mut self) -> Option<Runnable<TaskScheduleInfo>> {
        for q in self.queue.iter_mut() {
            let info = q.pop_front();
            if info.is_some() {
                return info;
            }
        }
        None
    }
}
lazy_static! {
    static ref EXECUTOR: SpinMutex<Executor> = SpinMutex::new(Executor::new());
}

/// insert task into EXECUTOR when [`core::task::Waker::wake`] get called
fn schedule(runnable: Runnable<TaskScheduleInfo>, info: ScheduleInfo) {
    trace!(
        "[sched] schedule task, prio: {}, woken_while_running: {}",
        runnable.metadata().prio(),
        info.woken_while_running
    );
    if info.woken_while_running {
        EXECUTOR.lock().push_front(runnable);
    } else {
        EXECUTOR.lock().push_back(runnable);
    }
}

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(future: F, prio: Arc<SyncUnsafeCell<isize>>)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (runnable, handle) = Builder::new()
        .metadata(TaskScheduleInfo::new(prio))
        .spawn(move |_: &TaskScheduleInfo| future, WithInfo(schedule));
    runnable.schedule();
    handle.detach();
}

/// Pop a task and run it
pub fn run() {
    // spin until find a valid task
    loop {
        let runnable = EXECUTOR.lock().pop_front();
        if let Some(runnable) = runnable {
            runnable.run();
            break;
        }
    }
}
