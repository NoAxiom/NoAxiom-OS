//! sync ref cell for multi-thread

use core::cell::{Ref, RefCell, RefMut};

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

unsafe impl<T> Sync for SyncRefCell<T> {
    // impl Sync for ref cell
    // can used in multi-thread
}

impl<T> SyncRefCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
}
