#![allow(unused)]

use alloc::{string::String, vec::Vec};

use super::address::VirtAddr;
use crate::{
    config::mm::KERNEL_ADDR_OFFSET,
    cpu::current_cpu,
    mm::address::{VirtPageNum, VpnRange},
};

/// check if the slice is well-allocated
/// any unallcated memory access will cause a page fault
/// and will be handled by the kernel_trap_handler => memory_validate
/// so we should validate the memory before we lock current memory_set
#[allow(unused)]
pub fn validate_slice(va: usize, len: usize) {
    warn!(
        "DON'T ABUSE THIS FUNCTION!!!\n
        [validate_slice] buf_addr = {:#x}, len = {:#x}",
        va, len
    );
    let start: VirtPageNum = VirtAddr::from(va).floor();
    let end: VirtPageNum = VirtAddr::from(va + len).ceil();
    let mut memory_set = current_cpu().task.as_ref().unwrap().memory_set().lock();
    for vpn in VpnRange::new(start, end) {
        let _ = memory_set.validate(vpn.as_va_usize(), None);
    }
}

/// the UserPtr is a wrapper for user-space pointer
/// NOTE THAT: it will NOT validate the pointer
/// and will probably trigger pagefault when accessing userspace
/// ## usage
/// complete any data clone before memory_set.lock
/// and write data after memory_set.unlock
/// ## example
/// ### clone data before memory_set.lock
/// ```
/// let addr = 0x1000;
/// let ptr = UserPtr::<u8>::new(addr);
/// let data_cloned = ptr.as_vec(); // this might trigger pagefault
/// let guard = memory_set.lock();
/// guard.write(data_cloned);
/// drop(guard);
/// ```
/// ### write data after memory_set.unlock
/// ```
/// let addr = 0x1000;
/// let ptr = UserPtr::<u8>::new(addr);
/// let guard = memory_set.lock();
/// let should_write_data = guard.read();
/// drop(guard);
/// let data_cloned = ptr.as_ref_mut(); // this might trigger pagefault
/// *data_cloned = should_write_data;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserPtr<T> {
    ptr: *mut T,
}

impl<T> UserPtr<T> {
    pub fn new(addr: usize) -> Self {
        assert!(
            addr & KERNEL_ADDR_OFFSET == 0,
            "shouldn't pass kernel address"
        );
        Self {
            ptr: addr as *mut T,
        }
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    #[inline(always)]
    pub fn inc(&mut self, count: usize) {
        self.ptr = unsafe { self.ptr.add(count) };
    }

    #[inline(always)]
    pub fn value(&self) -> T
    where
        T: Copy,
    {
        unsafe { *self.ptr }
    }

    #[inline(always)]
    pub fn addr(&self) -> usize {
        self.ptr as usize
    }

    #[inline(always)]
    pub unsafe fn set(&self, value: T) {
        unsafe { *self.ptr = value };
    }

    #[inline(always)]
    pub fn write_volatile(&self, value: T) {
        unsafe { self.ptr.write_volatile(value) };
    }

    /// convert ptr into an mutable reference
    /// please write data after memory_set.unlock
    #[inline(always)]
    pub fn as_ref_mut(&self) -> &mut T {
        unsafe { &mut *(self.ptr as *mut T) }
    }

    /// clone a slice as vec from user space
    pub fn as_vec(&self, len: usize) -> Vec<T>
    where
        T: Copy,
    {
        let mut ptr = self.ptr as usize;
        let mut res = Vec::with_capacity(len);
        let step = core::mem::size_of::<T>();
        trace!("[as_vec] ptr: {:#x}", ptr);
        for _ in 0..len {
            let value = unsafe { &*(ptr as *const T) };
            res.push(*value);
            ptr += step;
        }
        res
    }

    /// get user slice until the checker returns true
    pub fn as_vec_until(&self, checker: &dyn Fn(&T) -> bool) -> Vec<T>
    where
        T: Copy,
    {
        let mut ptr = self.ptr as usize;
        let mut res = Vec::new();
        let step = core::mem::size_of::<T>();
        loop {
            trace!("[as_vec_while] ptr: {:#x}", ptr);
            let value = unsafe { &*(ptr as *const T) };
            if checker(value) {
                break;
            }
            res.push(*value);
            ptr += step;
        }
        res
    }
}

impl UserPtr<u8> {
    /// get user string with length provided
    pub fn as_string(&self, len: usize) -> String {
        let vec = self.as_vec(len);
        let res = String::from_utf8(vec).unwrap();
        res
    }

    /// get user string
    pub fn get_cstr(&self) -> String {
        let checker = |&c: &u8| c as char == '\0';
        let slice = self.as_vec_until(&checker);
        trace!("slice: {:?}", slice);
        let res = String::from_utf8(Vec::from(slice)).unwrap();
        res
    }
}

impl UserPtr<UserPtr<u8>> {
    /// get user string vec, end with null
    pub fn get_string_vec(&self) -> Vec<String> {
        let mut ptr = self.clone();
        let mut res = Vec::new();
        while !ptr.is_null() && !ptr.value().is_null() {
            trace!(
                "ptr_addr: {:#}, value: {:#}",
                ptr.addr(),
                ptr.value().addr()
            );
            let data = ptr.value().get_cstr();
            res.push(data);
            ptr.inc(1);
        }
        res
    }
}

// the userptr is safe to send and sync
unsafe impl<T> Send for UserPtr<T> {}
unsafe impl<T> Sync for UserPtr<T> {}
