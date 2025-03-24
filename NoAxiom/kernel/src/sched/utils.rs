//! ## utils for async task
//! - use [`take_waker`] to fetch current task's context
//! - use [`block_on`] to block on a future
//! - use [`suspend_now`] to suspend current task (without immediate wake)
//! - use [`suspend_on`] to suspend current task on a future

use alloc::{boxed::Box, sync::Arc, task::Wake};
use core::{
    future::{poll_fn, Future},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use crate::{
    cpu::current_cpu,
    signal::sig_set::SigSet,
    task::{status::TaskStatus, Task},
};
pub struct YieldFuture {
    visited: bool,
}
impl YieldFuture {
    pub const fn new() -> Self {
        Self { visited: false }
    }
}
impl Future for YieldFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.visited {
            Poll::Ready(())
        } else {
            self.visited = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

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
    YieldFuture::new().await;
}

/// future to take the waker of the current task
struct TakeWakerFuture;
impl Future for TakeWakerFuture {
    type Output = Waker;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(cx.waker().clone())
    }
}

/// Take the waker of the current future
/// it won't change any schedule status,
/// since it returns Ready immediately
#[inline(always)]
#[allow(unused)]
pub async fn take_waker() -> Waker {
    TakeWakerFuture.await
}

/// BlockWaker do nothing since we always poll the future
struct BlockWaker;
impl Wake for BlockWaker {
    fn wake(self: Arc<Self>) {}
    fn wake_by_ref(self: &Arc<Self>) {}
}

/// Block on the future until it's ready.
/// Note that this function is used in kernel mode.
pub fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = Box::pin(future);
    let waker = Arc::new(BlockWaker).into();
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(res) = future.as_mut().poll(&mut cx) {
            return res;
        }
        // let mut k = get_time();
        // for i in 0..10000 {
        //     k *= (k + i);
        // }
        // intermit(|| info!("[block on] val is {}", k));
    }
}

pub struct SuspendFuture {
    visited: bool,
}

impl SuspendFuture {
    pub const fn new() -> Self {
        Self { visited: false }
    }
}

impl Future for SuspendFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
        match self.visited {
            true => Poll::Ready(()),
            false => {
                self.visited = true;
                Poll::Pending
            }
        }
    }
}

/// suspend current task
/// difference with yield_now: it won't wake the task immediately
pub async fn suspend_now() {
    let task = current_cpu().task.as_ref().unwrap();
    task.set_wake_signal(!*task.sig_mask());
    task.set_suspend();
    SuspendFuture::new().await;
    assert!(!task.is_suspend());
}

pub async fn suspend_now_with_sig(sig: SigSet) {
    let task = current_cpu().task.as_ref().unwrap();
    let sigset = !*task.sig_mask() | sig;
    task.set_wake_signal(sigset);
    task.set_suspend();
    SuspendFuture::new().await;
    assert!(!task.is_suspend());
}

impl Task {
    pub async fn suspend_on<T>(
        self: &Arc<Self>,
        future: impl Future<Output = T>,
        sig: Option<SigSet>,
    ) -> T {
        let mut future = Box::pin(future);
        let mut cnt = 0;
        let res = poll_fn(move |mut cx| match future.as_mut().poll(&mut cx) {
            Poll::Ready(res) => return Poll::Ready(res),
            Poll::Pending => {
                let mut pending = self.pending_sigs();
                let mut status = self.status();
                cnt += 1;
                warn!("task {} suspend on future, poll count: {}", self.tid(), cnt);
                let sigset = (!*self.sig_mask()) | (sig.unwrap_or_else(|| SigSet::empty()));
                trace!("wake set: {:?}", sigset);
                *status = TaskStatus::Suspend;
                pending.should_wake = sigset;
                drop(status);
                drop(pending);
                return Poll::Pending;
            }
        })
        .await;
        assert!(!self.is_suspend());
        res
    }
}

pub async fn suspend_on<T>(future: impl Future<Output = T>, sig: Option<SigSet>) -> T {
    let task = current_cpu().task.as_ref().unwrap();
    task.suspend_on(future, sig).await
}
