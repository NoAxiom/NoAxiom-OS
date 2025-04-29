use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use futures_lite::future::yield_now;

/// Async Mutex implemented with `AtomicBool`
///
/// When calling [`lock`], it will NOT block the current thread.
/// Instead, it will return a [`Future`] that can be awaited to avoid
/// wasting CPU time.
pub struct AsyncMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> AsyncMutex<T> {
    /// Create a new `AsyncMutex`
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Try to get the lock, returning `true` if successful
    pub fn try_lock(&self) -> Result<AsyncMutexGuard<'_, T>, ()> {
        // `Ordering::Acquire` gaurantees that the lock read is sync after the lock
        // write
        match self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            true => Ok(AsyncMutexGuard {
                data: unsafe { &mut *self.data.get() },
                lock: &self.locked,
            }),
            false => Err(()),
        }
    }

    /// Lock the source
    ///
    /// This will return a [`Future`] that can be awaited.
    pub async fn lock(&self) -> AsyncMutexGuard<'_, T> {
        loop {
            match self.try_lock() {
                Ok(guard) => return guard,
                Err(_) => {
                    // todo: save wakers and wake them up when unlocked, discard the `yield_now`
                    yield_now().await;
                }
            }
        }
    }
}

pub struct AsyncMutexGuard<'a, T> {
    data: &'a mut T,
    lock: &'a AtomicBool,
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
