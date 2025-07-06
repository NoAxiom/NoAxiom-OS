//! ## utils for async task
//! - use [`take_waker`] to fetch current task's context
//! - use [`block_on`] to block on a future
//! - use [`suspend_now`] to suspend current task (without immediate wake)

use alloc::sync::Arc;
use core::{future::Future, task::Waker};

pub use kfuture::block::block_on;
use kfuture::{suspend::SuspendFuture, take_waker::TakeWakerFuture, yield_fut::YieldFuture};
use ksync::assert_no_lock;

use crate::{cpu::current_task, task::Task};

impl Task {
    /// yield current task by awaiting this future,
    /// note that this should be wrapped in an async function,
    /// and will create an await point for the current task flow
    #[inline(always)]
    pub async fn yield_now(&self) {
        self.set_sched_prio_idle();
        self.clear_resched_flags();
        YieldFuture::new().await;
        self.set_sched_prio_normal();
    }
}

#[inline(always)]
pub async fn yield_now() {
    current_task().unwrap().yield_now().await;
}

/// Take the waker of the current future
/// it won't change any schedule status,
/// since it returns Ready immediately
#[inline(always)]
#[allow(unused)]
pub async fn take_waker() -> Waker {
    TakeWakerFuture.await
}

/// suspend current task
/// difference with yield_now: it won't wake the task immediately
pub async fn suspend_now() {
    assert_no_lock!();
    SuspendFuture::new().await;
}

pub async fn realtime<T>(task: &Arc<Task>, fut: impl Future<Output = T>) -> T {
    task.set_sched_prio_realtime(0);
    let res = fut.await;
    task.set_sched_prio_normal();
    res
}
