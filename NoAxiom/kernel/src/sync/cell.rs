//! sync ref cell for multi-thread

use core::cell::{Ref, RefCell, RefMut, UnsafeCell};

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
}

unsafe impl<T> Sync for SyncRefCell<T> {}

impl<T> SyncRefCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
}

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
}

unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            inner: UnsafeCell::new(value),
        }
    }
}
