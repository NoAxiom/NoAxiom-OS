use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use crate::mutex::SpinLock;

pub struct AsyncSpinLock<T> {
    inner: SpinLock<AsyncLockInner<T>>,
}

struct AsyncLockInner<T> {
    data: Option<T>,
    waker: Option<Waker>,
    locked: bool,
}

impl<T> AsyncSpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            inner: SpinLock::new(AsyncLockInner {
                data: Some(data),
                waker: None,
                locked: false,
            }),
        }
    }

    pub fn lock(&self) -> AsyncLockFuture<'_, T> {
        AsyncLockFuture { lock: self }
    }
}

pub struct AsyncLockFuture<'a, T> {
    lock: &'a AsyncSpinLock<T>,
}

impl<'a, T> Future for AsyncLockFuture<'a, T> {
    type Output = AsyncLockGuard<'a, T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.lock.inner.lock();
        if inner.locked {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        } else {
            inner.locked = true;
            Poll::Ready(AsyncLockGuard { lock: self.lock })
        }
    }
}

pub struct AsyncLockGuard<'a, T> {
    lock: &'a AsyncSpinLock<T>,
}

impl<'a, T> Drop for AsyncLockGuard<'a, T> {
    fn drop(&mut self) {
        let mut inner = self.lock.inner.lock();
        inner.locked = false;
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }
}

unsafe impl<T: Send> Send for AsyncSpinLock<T> {}
unsafe impl<T: Send> Sync for AsyncSpinLock<T> {}
