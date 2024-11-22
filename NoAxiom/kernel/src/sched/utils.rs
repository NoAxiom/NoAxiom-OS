//! ## utils for async task
//! - use [`yield_now`] to yield current async task;
//! - use [`take_waker`] to fetch current task's context

use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

struct YieldFuture {
    visited: bool,
}

impl YieldFuture {
    const fn new() -> Self {
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

/// yield current task by awaiting this future
#[inline(always)]
#[allow(unused)]
pub async fn yield_now() {
    YieldFuture::new().await;
}

struct TakeWakerFuture;

impl Future for TakeWakerFuture {
    type Output = Waker;
    #[inline(always)]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // immediately returns ready
        // so it won't change any scedule order
        Poll::Ready(cx.waker().clone())
    }
}

/// Take the waker of the current future
#[inline(always)]
#[allow(unused)]
pub async fn take_waker() -> Waker {
    TakeWakerFuture.await
}
