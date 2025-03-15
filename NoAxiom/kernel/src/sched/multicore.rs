//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use core::sync::atomic::{AtomicUsize, Ordering};

use array_init::array_init;
use async_task::{Runnable, ScheduleInfo};
use ksync::mutex::SpinLock;

use super::{
    sched_info::SchedInfo,
    vsched::{MulticoreRuntime, MulticoreScheduler, Runtime, ScheduleOrder::*},
};
use crate::{
    config::{arch::CPU_NUM, sched::TIME_SLICE_TICKS},
    cpu::get_hartid,
    time::sleep::block_on_sleep,
};

pub struct NoAxiomRuntime<T>
where
    T: MulticoreScheduler<SchedInfo>,
{
    /// use cpu mask to pass request
    sched_req: AtomicUsize,

    /// the load sum of all cores
    all_load: AtomicUsize,

    /// scheduler for each core
    scheduler: [SpinLock<T>; CPU_NUM],
}

impl<T> NoAxiomRuntime<T>
where
    T: MulticoreScheduler<SchedInfo>,
{
    fn current_scheduler<'a>(&self) -> &SpinLock<T> {
        &self.scheduler[get_hartid()]
    }
}

impl<T> MulticoreRuntime<T, SchedInfo> for NoAxiomRuntime<T>
where
    Self: Runtime<T, SchedInfo>,
    T: MulticoreScheduler<SchedInfo>,
{
    fn add_load(&self, load: usize) {
        self.all_load.fetch_add(load, Ordering::AcqRel);
    }
    fn sub_load(&self, load: usize) {
        self.all_load.fetch_sub(load, Ordering::AcqRel);
    }
    fn all_load(&self) -> usize {
        self.all_load.load(Ordering::Acquire)
    }
}

impl<T> Runtime<T, SchedInfo> for NoAxiomRuntime<T>
where
    T: MulticoreScheduler<SchedInfo>,
{
    fn new() -> Self {
        Self {
            sched_req: AtomicUsize::new(0),
            all_load: AtomicUsize::new(0),
            scheduler: array_init(|_| SpinLock::new(T::default())),
        }
    }

    fn schedule(&self, runnable: Runnable<SchedInfo>, info: ScheduleInfo) {
        self.current_scheduler()
            .lock()
            .push_with_info(runnable, info);
    }

    fn run(&self) {
        let mut local = self.current_scheduler().lock();
        // check load balance
        let all_load = self.all_load(); // safe, it does not affect the correctness
        if local.is_timeup() {
            local.set_last_time();
            if local.is_underload(all_load) {
                for i in 0..CPU_NUM {
                    if i == get_hartid() {
                        continue;
                    }
                    let mut other = self.scheduler[i].lock();
                    if other.is_overload(all_load) {
                        while other.is_overload(all_load) && local.is_underload(all_load) {
                            if let Some(runnable) = other.pop(NormalFirst) {
                                local.push_normal(runnable);
                            } else {
                                break;
                            }
                        }
                        if !local.is_underload(all_load) {
                            break;
                        }
                    }
                }
            }
        }
        let runnable = local.pop(UrgentFirst);
        drop(local);
        if let Some(runnable) = runnable {
            runnable.run();
        } else {
            block_on_sleep(TIME_SLICE_TICKS / 10);
        }
    }
}
