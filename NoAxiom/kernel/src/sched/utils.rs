//! ## utils for async task
//! - use [`take_waker`] to fetch current task's context
//! - use [`block_on`] to block on a future
//! - use [`suspend_now`] to suspend current task (without immediate wake)

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use include::errno::Errno;
pub use kfuture::block::block_on;
use kfuture::{suspend::SuspendFuture, take_waker::TakeWakerFuture, yield_fut::YieldFuture};
use ksync::mutex::check_no_lock;
use pin_project_lite::pin_project;

use crate::{cpu::current_task, signal::sig_set::SigMask, syscall::SysResult, task::Task};

impl Task {
    /// yield current task by awaiting this future,
    /// note that this should be wrapped in an async function,
    /// and will create an await point for the current task flow
    #[inline(always)]
    pub async fn yield_now(&self) {
        YieldFuture::new().await;
    }
}

#[inline(always)]
pub async fn yield_now() {
    current_task().unwrap().set_sched_prio_idle();
    YieldFuture::new().await;
    current_task().unwrap().set_sched_prio_normal();
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
    assert!(check_no_lock());
    SuspendFuture::new().await;
}

pin_project! {
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct IntableFuture<'a, F> {
        task: &'a Arc<Task>,
        #[pin]
        fut: F,
        mask: Option<SigMask>,
    }
}

impl<F, T> Future for IntableFuture<'_, F>
where
    F: Future<Output = T>,
{
    type Output = SysResult<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let task = this.task;
        match this.fut.poll(cx) {
            Poll::Ready(res) => Poll::Ready(Ok(res)),
            Poll::Pending => {
                // start to handle signal
                if task.peek_has_pending_signal(this.mask) {
                    Poll::Ready(Err(Errno::EINTR))
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

pub async fn intable<T>(
    task: &Arc<Task>,
    fut: impl Future<Output = T>,
    block_sig: Option<SigMask>,
) -> SysResult<T> {
    IntableFuture {
        task,
        fut,
        mask: block_sig,
    }
    .await
}

pub async fn realtime<T>(task: &Arc<Task>, fut: impl Future<Output = T>) -> T {
    task.set_sched_prio_realtime(0);
    let res = fut.await;
    task.set_sched_prio_normal();
    res
}
