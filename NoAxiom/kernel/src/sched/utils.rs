//! ## utils for async task
//! - use [`yield_now`] to yield current async task;
//! - use [`take_waker`] to fetch current task's context

#![allow(unused)]

use alloc::{boxed::Box, sync::Arc, task::Wake};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
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

/// yield current task by awaiting this future,
/// note that this should be wrapped in an async function,
/// and will create an await point for the current task flow
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
    }
}
