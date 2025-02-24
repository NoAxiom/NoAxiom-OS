//! sync ref cell for multi-thread

use core::{
    cell::{Ref, RefCell, RefMut, UnsafeCell},
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct SyncRefCell<T> {
    inner: RefCell<T>,
}

impl<T> SyncRefCell<T> {
    pub fn borrow(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
    pub const fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
}

unsafe impl<T> Sync for SyncRefCell<T> {}

/// sync unsafe cell for multi-thread,
/// this cell is provided for thread resources sharing,
/// it won't lead to real multi-threading since thread:hart are 1:1 mapped
pub struct SyncUnsafeCell<T> {
    inner: UnsafeCell<T>,
}

impl<T> SyncUnsafeCell<T> {
    pub fn get(&self) -> *mut T {
        self.inner.get()
    }
    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut()
    }
    pub const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}

unsafe impl<T> Sync for SyncUnsafeCell<T> {}

pub struct AtomicSyncUnsafeCell<T> {
    inner: AtomicUsize,
    _phantom: core::marker::PhantomData<T>,
}

impl<T> AtomicSyncUnsafeCell<T> {
    pub fn get(&self) -> &T {
        unsafe { &*(self.inner.load(Ordering::Relaxed) as *const T) }
    }
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *(self.inner.load(Ordering::Relaxed) as *mut T) }
    }
    pub fn update(&self, value: T) {
        self.inner
            .store(&value as *const T as usize, Ordering::Relaxed);
    }
}
