use alloc::sync::Arc;

use arch::{Arch, ArchInt, ArchMemory};
use async_task::{Builder, Runnable, WithInfo};
use config::task::INIT_PROCESS_ID;
use ksync::mutex::SpinLock;
use lazy_static::lazy_static;

use super::{
    sched_entity::SchedMetadata,
    scheduler::{Info, MultiLevelScheduler},
    vsched::{Runtime, Scheduler},
};
use crate::{
    cpu::{get_hartid, CPUS},
    task::{manager::TASK_MANAGER, Task},
    time::{gettime::get_time_duration, timer::timer_handler},
};

type SchedulerImpl = MultiLevelScheduler;
pub struct SimpleRuntime {
    scheduler: SpinLock<SchedulerImpl>,
}

impl Runtime<SchedulerImpl, Info> for SimpleRuntime {
    fn new() -> Self {
        Self {
            scheduler: SpinLock::new(SchedulerImpl::new()),
        }
    }
    fn run(&self) {
        let runnable = self.scheduler.lock().pop();
        if let Some(runnable) = runnable {
            runnable.run();
        }
    }
    fn schedule(&self, runnable: Runnable<Info>, info: async_task::ScheduleInfo) {
        self.scheduler.lock().push(runnable, info);
    }
    fn spawn<F>(self: &'static Self, future: F, task: Option<&Arc<Task>>)
    where
        F: core::future::Future<Output: Send + 'static> + Send + 'static,
    {
        let metadata = task
            .map(|task| SchedMetadata::from_task(task))
            .unwrap_or_else(SchedMetadata::default);
        let (runnable, handle) = Builder::new().metadata(metadata).spawn(
            move |_| future,
            WithInfo(move |runnable, info| self.schedule(runnable, info)),
        );
        runnable.schedule();
        handle.detach();
    }
}

type RuntimeImpl = SimpleRuntime;
lazy_static! {
    pub static ref RUNTIME: RuntimeImpl = RuntimeImpl::new();
}

/// run_tasks: only act as a task runner
#[no_mangle]
pub fn run_tasks() -> ! {
    info!("[kernel] hart {} has been booted", get_hartid());
    loop {
        assert!(Arch::is_interrupt_enabled());
        timer_handler();
        RUNTIME.run();
        // context_switch_test();
    }
}

#[allow(unused)]
fn context_switch_test() {
    if let Some(init_proc) = TASK_MANAGER.get(INIT_PROCESS_ID) {
        let time0 = get_time_duration();
        const NUM: usize = 100000;
        let mut counter = 0;
        for i in 0..NUM {
            CPUS[get_hartid()].as_ref_mut().set_task(&init_proc);
            counter += i;
            CPUS[get_hartid()].as_ref_mut().clear_task();
        }
        let time1 = get_time_duration();
        for i in 0..NUM {
            Arch::tlb_flush();
            counter += i;
            Arch::tlb_flush();
        }
        let time2 = get_time_duration();
        for i in 0..NUM {
            counter += i;
        }
        let time3 = get_time_duration();
        println!(
            "[kernel] hart {} switch time: {:?}, flush time: {:?}, arith time: {:?}, n: {}, counter: {}",
            get_hartid(),
            time1 - time0,
            time2 - time1,
            time3 - time2,
            NUM,
            counter
        );
    }
}
