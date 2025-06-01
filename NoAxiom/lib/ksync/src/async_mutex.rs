use alloc::collections::vec_deque::VecDeque;
use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
    task::Waker,
};

use kfuture::{suspend::SuspendFuture, take_waker::TakeWakerFuture};

use crate::mutex::SpinLock;

/// Async Mutex implemented with `AtomicBool`
///
/// When calling [`lock`], it will NOT block the current thread.
/// Instead, it will return a [`Future`] that can be awaited to avoid
/// wasting CPU time.
pub struct AsyncMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
    waiters: SpinLock<VecDeque<Waker>>,
}

impl<T> AsyncMutex<T> {
    /// Create a new `AsyncMutex`
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            waiters: SpinLock::new(VecDeque::new()),
        }
    }

    /// Try to get the lock, returning `true` if successful
    pub fn try_lock(&self) -> Option<AsyncMutexGuard<'_, T>> {
        // `Ordering::Acquire` gaurantees that the lock read is sync after the lock
        // write
        match self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            true => Some(AsyncMutexGuard {
                data: unsafe { &mut *self.data.get() },
                lock: &self.locked,
                waiters: &self.waiters,
            }),
            false => None,
        }
    }

    /// Lock the source
    ///
    /// This will return a [`Future`] that can be awaited.
    pub async fn lock(&self) -> AsyncMutexGuard<'_, T> {
        loop {
            match self.try_lock() {
                // Some(guard) => return guard,
                // None => {
                //     kfuture::yield_fut::YieldFuture::new().await;
                // }
                Some(guard) => return guard,
                None => {
                    self.waiters.lock().push_back(TakeWakerFuture.await);
                    SuspendFuture::new().await;
                }
            }
        }
    }
}

pub struct AsyncMutexGuard<'a, T> {
    data: &'a mut T,
    lock: &'a AtomicBool,
    waiters: &'a SpinLock<VecDeque<Waker>>,
}

impl<T> Deref for AsyncMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        return self.data;
    }
}

impl<T> DerefMut for AsyncMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return &mut *self.data;
    }
}

impl<T> Drop for AsyncMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.waiters.lock().pop_front().map(|waker| waker.wake());
        self.lock.store(false, Ordering::Release);
    }
}

unsafe impl<T> Sync for AsyncMutex<T> {}

mod test {
    #[test]
    fn test_async_mutex() {
        for i in 0..10000 {
            spawn_ktask(async move {
                let mut guard = SOURCE.lock().await;
                let mut k = 998244353;
                for j in 0..118713 {
                    k = k * (k + j + i);
                    if (i * j) % 97572 == 3572 {
                        // use sleep
                        yield_now().await;
                    }
                }
                debug!("[asyncmutex]count: {}, hash: {}", *guard, k);
                *guard += 1;
            });
        }
    }
}
