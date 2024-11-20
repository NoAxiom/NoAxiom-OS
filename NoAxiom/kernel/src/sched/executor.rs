//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use log::info;
use core::future::Future;

use async_task::{Builder, Runnable, ScheduleInfo, WithInfo};
use lazy_static::lazy_static;
use log::info;

use crate::{
    config::sched::MLFQ_LEVELS,
    sync::{cell::SyncRefCell, mutex::SpinMutex},
};

struct TaskScheduleInfoInner {
    prio: usize,
}
impl TaskScheduleInfoInner {
    pub const fn new(prio: usize) -> Self {
        Self { prio }
    }
    pub fn level(&self) -> usize {
        self.prio
    }
    pub fn update(&mut self) {
        self.prio = self.prio + 1;
    }
}

struct TaskScheduleInfo {
    inner: SyncRefCell<TaskScheduleInfoInner>,
}
impl TaskScheduleInfo {
    pub const fn new(prio: usize) -> Self {
        Self {
            inner: SyncRefCell::new(TaskScheduleInfoInner::new(prio)),
        }
    }
    pub fn level(&self) -> usize {
        self.inner.borrow().level()
    }
    pub fn update(&self) {
        self.inner.borrow_mut().update();
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
        let level = runnable.metadata().level();
        info!("[sched] push task to back, prio: {}", level);
        self.queue[level].push_back(runnable);
    }
    fn push_front(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        let level = runnable.metadata().level();
        info!("[sched] push task to front, prio: {}", level);
        self.queue[level].push_front(runnable);
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
fn schedule(task: Runnable<TaskScheduleInfo>, info: ScheduleInfo) {
    info!("[sched] schedule task, prio: {}", task.metadata().level());
    task.metadata().update();
    info!(
        "[sched] schedule task, new prio: {}",
        task.metadata().level()
    );
    info!(
        "[sched] schedule task, woken_while_running: {}",
        info.woken_while_running
    );
    if info.woken_while_running {
        EXECUTOR.lock().push_front(task);
    } else {
        EXECUTOR.lock().push_back(task);
    }
}

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    // let (task, handle) = async_task::spawn(future, WithInfo(schedule));
    let (task, handle) = Builder::new()
        .metadata(TaskScheduleInfo::new(0))
        .spawn(move |_: &TaskScheduleInfo| future, WithInfo(schedule));
    task.schedule();
    handle.detach();
}

/// Pop a task and run it
pub fn run() {
    // spin until find a valid task
    loop {
        let task = EXECUTOR.lock().pop_front();
        if let Some(task) = task {
            info!("[sched] run task, prio: {}", task.metadata().level());
            task.run();
            info!("[sched] task done");
            break;
        } else {
            crate::driver::sbi::shutdown();
            // todo!()
        }
    }
}
