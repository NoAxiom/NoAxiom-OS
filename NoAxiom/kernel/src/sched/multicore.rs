use array_init::array_init;
use async_task::{Runnable, ScheduleInfo};
use ksync::mutex::SpinLock;

use super::{
    sched_info::SchedInfo,
    vsched::{MulticoreRuntime, MulticoreScheduler, Runtime, ScheduleOrder::*},
};
use crate::{
    config::arch::CPU_NUM, constant::sched::NICE_0_LOAD, cpu::get_hartid,
    time::sleep::current_sleep_manager,
};

pub struct NoAxiomRuntime<T>
where
    T: MulticoreScheduler<SchedInfo>,
{
    /// load_balance marker
    load_balance_lock: SpinLock<()>,

    /// scheduler for each core
    scheduler: [SpinLock<T>; CPU_NUM],
}

impl<T> NoAxiomRuntime<T>
where
    T: MulticoreScheduler<SchedInfo>,
{
    fn current_scheduler<'a>(&self) -> &SpinLock<T> {
        &self.scheduler[0]
    }
    #[allow(unused)]
    fn load_balance(&self) -> bool {
        #[derive(Clone, Copy, Default)]
        struct LoadInfo {
            hart: usize,
            load: usize,
            count: usize,
            is_running: bool,
        }
        impl LoadInfo {
            fn calc_load(&self) -> usize {
                let addition = match self.is_running {
                    true => NICE_0_LOAD,
                    false => 0,
                };
                self.load + addition
            }
            fn is_valid(&self) -> bool {
                match self.is_running {
                    true => self.count > 0,
                    false => self.count > 1,
                }
            }
        }
        let mut load_info = [LoadInfo::default(); CPU_NUM];
        for i in 0..CPU_NUM {
            let other = self.scheduler[i].lock();
            load_info[i] = LoadInfo {
                hart: i,
                load: other.load(),
                count: other.task_count(),
                is_running: other.is_running(),
            };
        }
        let max_info = *load_info
            .iter()
            .max_by(|l, r| l.calc_load().cmp(&r.calc_load()))
            .unwrap();
        if max_info.hart == get_hartid() || !max_info.is_valid() {
            return false;
        }

        let guard = self.load_balance_lock.lock();
        let mut local = self.current_scheduler().lock();
        let mut other = self.scheduler[max_info.hart].lock();
        let addition = match other.is_running() {
            true => NICE_0_LOAD,
            false => 0,
        };
        warn!(
            "load balance from {} to {}, local_load: {}, other_load: {}, other_is_running: {}",
            max_info.hart,
            get_hartid(),
            local.load(),
            other.load(),
            other.is_running()
        );
        while local.load() + NICE_0_LOAD < other.load() + addition && other.task_count() > 0 {
            let runnable = other.pop(NormalFirst);
            if let Some(runnable) = runnable {
                local.push_normal(runnable);
            } else {
                break;
            }
        }
        drop(guard);
        true
    }
}

impl<T> MulticoreRuntime<T, SchedInfo> for NoAxiomRuntime<T>
where
    Self: Runtime<T, SchedInfo>,
    T: MulticoreScheduler<SchedInfo>,
{
}

impl<T> Runtime<T, SchedInfo> for NoAxiomRuntime<T>
where
    T: MulticoreScheduler<SchedInfo>,
{
    fn new() -> Self {
        Self {
            load_balance_lock: SpinLock::new(()),
            scheduler: array_init(|_| SpinLock::new(T::default())),
        }
    }

    fn schedule(&self, runnable: Runnable<SchedInfo>, info: ScheduleInfo) {
        let mut local = self.current_scheduler().lock();
        local.push_with_info(runnable, info);
    }

    fn run(&self) {
        current_sleep_manager().sleep_handler();
        let mut local = self.current_scheduler().lock();
        local.set_running(false);
        // run task
        let runnable = local.pop(UrgentFirst);
        if let Some(runnable) = runnable {
            local.set_running(true);
            drop(local);
            runnable.run();
        } else {
            // #[cfg(feature = "multicore")]
            // if local.is_timeup() {
            //     // timeup, check load balance
            //     trace!("timeup detected!");
            //     local.set_last_time();
            //     drop(local);
            //     if !self.load_balance() {
            //         local.set_time_limit(LOAD_BALANCE_TICKS * 5);
            //     }
            // }
        }
    }
}
