use alloc::{boxed::Box, sync::Arc, task::Wake};
use core::{
    future::Future,
    task::{Context, Poll},
};

/// BlockWaker do nothing since we always poll the future
struct BlockWaker;
impl Wake for BlockWaker {
    fn wake(self: Arc<Self>) {}
    fn wake_by_ref(self: &Arc<Self>) {}
}

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
