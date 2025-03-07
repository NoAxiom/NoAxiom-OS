//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use arch::{Arch, ArchAsm, ArchInt, ArchSbi};
use array_init::array_init;
use async_task::{Runnable, ScheduleInfo};
use ksync::{cell::SyncUnsafeCell, mutex::SpinLock};
use lazy_static::lazy_static;

use super::{
    cfs::CFS,
    sched_entity::{SchedEntity, SchedTaskInfo},
    scheduler::Scheduler,
};
use crate::{
    config::{arch::CPU_NUM, sched::LOAD_BALANCE_TICKS},
    cpu::get_hartid,
    time::gettime::get_time,
    trap::ipi::{send_ipi, IpiType},
};

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

pub type RunnableTask = Runnable<TaskScheduleInfo>;
pub struct Runtime<T: Scheduler> {
    /// global task queue
    global_tasks: SpinLock<Vec<RunnableTask>>,

    /// use cpu mask to pass request
    sched_req: AtomicUsize,

    /// the load sum of all cores
    all_load: AtomicUsize,

    /// last contribution time
    last_push_time: [SyncUnsafeCell<usize>; CPU_NUM],

    /// last request time
    last_req_time: [SyncUnsafeCell<usize>; CPU_NUM],

    /// scheduler for each core
    scheduler: [SyncUnsafeCell<T>; CPU_NUM],
}

impl<T> Runtime<T>
where
    T: Scheduler,
{
    pub fn new() -> Self {
        Self {
            global_tasks: SpinLock::new(Vec::new()),
            sched_req: AtomicUsize::new(0),
            all_load: AtomicUsize::new(0),
            last_push_time: array_init(|_| SyncUnsafeCell::new(0)),
            last_req_time: array_init(|_| SyncUnsafeCell::new(0)),
            scheduler: array_init(|_| SyncUnsafeCell::new(T::default())),
        }
    }

    pub fn current_scheduler(&self) -> &mut T {
        unsafe { &mut *self.scheduler[get_hartid()].get() }
    }

    pub fn push(&self, runnable: RunnableTask, info: ScheduleInfo) {
        self.current_scheduler().push(runnable, info);
    }
    pub fn pop(&self) -> Option<RunnableTask> {
        self.current_scheduler().pop()
    }

    pub fn set_sched_req(&self) {
        let mask = 1 << get_hartid();
        self.sched_req.fetch_or(mask, Ordering::SeqCst);
        self.set_last_req_time(get_time());
    }

    pub fn get_load(&self) -> usize {
        RUNTIME.all_load.load(core::sync::atomic::Ordering::SeqCst)
    }
    pub fn add_load(&self, load: usize) {
        self.all_load.fetch_add(load, Ordering::SeqCst);
    }
    pub fn sub_load(&self, load: usize) {
        self.all_load.fetch_sub(load, Ordering::SeqCst);
    }

    pub fn last_push_time(&self) -> usize {
        unsafe { *self.last_push_time[get_hartid()].get() }
    }
    pub fn last_req_time(&self) -> usize {
        unsafe { *self.last_req_time[get_hartid()].get() }
    }
    pub fn set_last_push_time(&self, time: usize) {
        *&mut unsafe { *self.last_push_time[get_hartid()].get() } = time;
    }
    pub fn set_last_req_time(&self, time: usize) {
        *&mut unsafe { *self.last_req_time[get_hartid()].get() } = time;
    }

    pub fn current_is_overload(&self) -> bool {
        let is_timeup = get_time() - self.last_push_time() > LOAD_BALANCE_TICKS;
        is_timeup && self.current_scheduler().is_overload()
    }
    pub fn current_is_underload(&self) -> bool {
        get_time() - self.last_req_time() > LOAD_BALANCE_TICKS
            && self.current_scheduler().is_underload()
    }

    pub fn try_respond_sched_req(&self) {
        warn!("try_respond_sched_req");
        let cur_hartid = get_hartid();
        for i in 0..CPU_NUM {
            if i == cur_hartid {
                continue;
            }
            let mask = 1 << i;
            let val = self.sched_req.fetch_and(!mask, Ordering::SeqCst);
            if val & mask != 0 {
                // request detected, now push tasks
                while self.current_is_overload() {
                    if let Some(runnable) = self.pop() {
                        self.global_tasks.lock().push(runnable);
                    } else {
                        break;
                    }
                }
                warn!("load_balance done! now wake up hartid: {}", i);
                self.set_last_push_time(get_time());
                send_ipi(i, IpiType::LoadBalance);
                return;
            }
        }
        warn!("load_balance failed");
    }
}

// TODO: add muticore support
lazy_static! {
    pub static ref RUNTIME: Runtime<CFS> = Runtime::new();
}

/// when other core detect a load imbalance, it will send a IPI to this core
/// and then the current core will enter this function to fetch global tasks
pub fn load_balance_handler() {
    let mut global = RUNTIME.global_tasks.lock();
    let local = RUNTIME.current_scheduler();
    while let Some(task) = global.pop() {
        local.push_normal(task);
    }
}

/// Pop a task and run it
pub fn run() {
    assert!(Arch::is_interrupt_enabled());
    // Arch::enable_global_interrupt();
    // spin until find a valid task
    let runnable = RUNTIME.pop();
    if let Some(runnable) = runnable {
        runnable.run();
    } else if RUNTIME.current_is_underload() {
        #[cfg(feature = "multicore")]
        {
            // fixme: why did set_idle not work???
            RUNTIME.set_sched_req();
            assert!(Arch::is_interrupt_enabled());
            return;
        }
    }
    #[cfg(feature = "multicore")]
    {
        if RUNTIME.current_is_overload() {
            // if current core is overload, try to respond request from other cores
            RUNTIME.try_respond_sched_req();
        } else if RUNTIME.current_is_underload() {
            RUNTIME.set_sched_req();
        }
    }
}
