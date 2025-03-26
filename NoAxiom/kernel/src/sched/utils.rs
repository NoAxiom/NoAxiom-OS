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

use ksync::mutex::{check_no_lock, SpinLockGuard};

use crate::{
    cpu::current_cpu,
    signal::sig_set::SigSet,
    task::{status::TaskStatus, Task, TaskInner},
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
/// WARNING: don't use it to wrap a bare suspend_now future
/// if used, you should wrap the suspend_now in another loop checker
pub fn block_on<T>(future: impl Future<Output = T>) -> T {
    let mut future = Box::pin(future);
    let waker = Arc::new(BlockWaker).into();
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(res) = future.as_mut().poll(&mut cx) {
            return res;
        }
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

#[inline(always)]
fn current_set_runnable() {
    current_cpu().task.as_ref().unwrap().pcb().set_runnable();
}

pub async fn suspend_no_int_now(mut pcb: SpinLockGuard<'_, TaskInner>) {
    pcb.set_status(TaskStatus::SuspendNoInt);
    drop(pcb);
    SuspendFuture::new().await;
    current_set_runnable();
}

/// suspend current task
/// difference with yield_now: it won't wake the task immediately
pub async fn suspend_now(mut pcb: SpinLockGuard<'_, TaskInner>) {
    let mask = pcb.pending_sigs.sig_mask;
    pcb.set_wake_signal(!mask);
    pcb.set_suspend();
    drop(pcb);
    SuspendFuture::new().await;
    current_set_runnable();
}

pub async fn suspend_now_with_sig(mut pcb: SpinLockGuard<'_, TaskInner>, sig: SigSet) {
    let sigset = (!pcb.sig_mask()) | sig;
    pcb.set_wake_signal(sigset);
    pcb.set_suspend();
    drop(pcb);
    SuspendFuture::new().await;
    current_set_runnable();
}

impl Task {
    pub async fn suspend_on<T>(
        self: &Arc<Self>,
        mut pcb: SpinLockGuard<'_, TaskInner>,
        future: impl Future<Output = T>,
        sig: Option<SigSet>,
    ) -> T {
        let sigset = (!pcb.sig_mask()) | (sig.unwrap_or_else(|| SigSet::empty()));
        pcb.set_wake_signal(sigset);
        pcb.set_suspend();
        drop(pcb);
        let res = future.await;
        current_set_runnable();
        res
    }
}

pub async fn suspend_on<T>(
    future: impl Future<Output = T>,
    sig: Option<SigSet>,
) -> T {
    let task = current_cpu().task.as_ref().unwrap();
    assert!(check_no_lock());
    task.suspend_on(task.pcb(), future, sig).await
}
