use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

/// future to take the waker of the current task
pub struct TakeWakerFuture;
impl Future for TakeWakerFuture {
    type Output = Waker;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(cx.waker().clone())
    }
}
