use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// future which will yield current task
/// it will wake the current task immediately
/// and the yield behaviour depends on the scheduler
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
