//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::{collections::vec_deque::VecDeque, sync::Arc, vec::Vec};
use core::future::Future;

use async_task::{Builder, Runnable, ScheduleInfo, WithInfo};
use lazy_static::lazy_static;

use crate::{
    config::sched::MLFQ_LEVELS, sync::mutex::SpinMutex, task::Task,
    time::timer::set_next_trigger,
};

pub struct TaskScheduleInfo {
    task: Option<Arc<Task>>,
}
impl TaskScheduleInfo {
    pub const fn new(task: Option<Arc<Task>>) -> Self {
        Self { task }
    }
    pub fn prio(&self) -> isize {
        if let Some(task) = &self.task {
            // todo: discard mlfq and use cfg
            let level = task.prio();
            if level < MLFQ_LEVELS as isize {
                level as isize
            } else {
                MLFQ_LEVELS as isize - 1
            }
        } else {
            0
        }
    }
    #[allow(unused)]
    pub fn update(&mut self, prio: isize) {
        info!("update task prio");
        if let Some(task) = &self.task {
            task.set_prio(prio);
        }
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
        info!("[sched] push task to back, prio: {}", level);
        self.queue[level as usize].push_back(runnable);
    }
    fn push_front(&mut self, runnable: Runnable<TaskScheduleInfo>) {
        let level = runnable.metadata().prio();
        info!("[sched] push task to front, prio: {}", level);
        self.queue[level as usize].push_front(runnable);
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
    info!("[sched] schedule task, prio: {}", task.metadata().prio());
    info!(
        "[sched] schedule task, new prio: {}",
        task.metadata().prio()
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
pub fn spawn_raw<F, R>(future: F, task: Option<Arc<Task>>)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (task, handle) = Builder::new()
        .metadata(TaskScheduleInfo::new(task))
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
            info!("[sched] run task, prio: {}", task.metadata().prio());
            set_next_trigger();
            task.run();
            info!("[sched] task done");
            break;
        }
    }
}
