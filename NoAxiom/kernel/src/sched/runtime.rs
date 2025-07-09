use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use core::time::Duration;

use arch::{Arch, ArchInt};
use async_task::{Builder, Runnable, WithInfo};
use ksync::mutex::SpinLock;
use lazy_static::lazy_static;

use super::{
    sched_entity::SchedMetadata,
    scheduler::{Info, MultiLevelScheduler},
    vsched::{Runtime, Scheduler},
};
use crate::{
    cpu::get_hartid,
    task::Task,
    time::{
        time_slice::{set_next_trigger, TimeSliceInfo},
        timer::timer_handler,
    },
};

type SchedulerImpl = MultiLevelScheduler;
pub struct MultiLevelRuntime {
    scheduler: SpinLock<SchedulerImpl>,
}

impl Runtime<Info> for MultiLevelRuntime {
    fn new() -> Self {
        Self {
            scheduler: SpinLock::new(SchedulerImpl::new()),
        }
    }
    fn run(&self) {
        let runnable = self.scheduler.lock().pop();
        if let Some(runnable) = runnable {
            set_next_trigger(None);
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

impl MultiLevelRuntime {
    pub fn handle_realtime(&self) {
        let mut sched = self.scheduler.lock();
        let mut tasks = VecDeque::new();
        while let Some(task) = sched.pop_realtime() {
            tasks.push_back(task);
        }
        drop(sched);
        assert_no_lock!();
        for task in tasks {
            set_next_trigger(Some(TimeSliceInfo::realtime()));
            task.run();
        }
    }
}

type RuntimeImpl = MultiLevelRuntime;
lazy_static! {
    pub static ref RUNTIME: RuntimeImpl = RuntimeImpl::new();
}

/// run_tasks: only act as a task runner
#[no_mangle]
pub fn run_tasks() -> ! {
    info!("[kernel] hart {} has been booted", get_hartid());
    loop {
        timer_handler();
        Arch::enable_interrupt();
        #[cfg(feature = "debug_sig")]
        {
            use crate::utils::crossover::intermit;
            intermit(Some(10000000), Some(Duration::from_secs(1)), || {
                memory::utils::print_mem_info();
                if let Some(manager) = crate::task::manager::TASK_MANAGER.0.try_lock() {
                    for (id, task) in manager.iter() {
                        if let Some(task) = task.upgrade() {
                            assert!(task.tid() == *id);
                            let pcb = task.pcb();
                            warn!(
                                "[main] tid{} in {:?}, pending_sig: {:?}, pending_set: {}, should_wake: {}, mask: {}",
                                task.tid(),
                                task.tcb().current_syscall,
                                pcb.signals
                                    .queue
                                    .iter()
                                    .map(|s| s.signal)
                                    .collect::<alloc::vec::Vec<_>>(),
                                pcb.signals.pending_set.debug_info_short(),
                                pcb.signals.should_wake.debug_info_short(),
                                task.sig_mask().debug_info_short(),
                            );
                        } else {
                            error!("[main] tid{} NOT FOUND!!!", id);
                        }
                    }
                } else {
                    error!("[main] task manager got locked!");
                }
            });
        }
        RUNTIME.run();
    }
}
