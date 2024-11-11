use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
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

pub async fn yield_current() {
    YieldFuture::new().await;
}
