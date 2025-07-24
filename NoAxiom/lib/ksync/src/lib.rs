#![no_std]
pub mod async_lock;
pub mod async_mutex;
pub mod cell;
pub mod mutex;

pub use spin::{Lazy, Once};

extern crate alloc;

pub use async_lock::{
    // Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard
    RwLock as AsyncRwLock,
    RwLockReadGuard as AsyncRwLockReadGuard,
    RwLockWriteGuard as AsyncRwLockWriteGuard,
};
pub use async_mutex::{AsyncMutex, AsyncMutexGuard};
