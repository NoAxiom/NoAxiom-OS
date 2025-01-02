use super::address::VirtAddr;
use crate::{
    cpu::current_cpu,
    mm::address::{VirtPageNum, VpnRange},
};

/// check if the slice is well-allocated
/// any unallcated memory access will cause a page fault
/// and will be handled by the kernel_trap_handler => memory_validate
/// so we should validate the memory before we lock current memory_set
fn validate_slice(va: usize, len: usize) {
    trace!("[validate_slice] buf_addr = {:#x}, len = {:#x}", va, len);
    let start: VirtPageNum = VirtAddr::from(va).floor();
    let end: VirtPageNum = VirtAddr::from(va + len).ceil();
    let task = current_cpu().task.as_ref().unwrap();
    for vpn in VpnRange::new(start, end) {
        let _ = task.memory_validate(vpn.as_va_usize());
    }
}

/// the UserPtr is a wrapper for user-space pointer
/// it will validate the pointer before we access it
pub struct UserPtr<T> {
    ptr: *const T,
}

#[allow(unused)]
impl<T> UserPtr<T> {
    pub fn new(addr: usize) -> Self {
        validate_slice(addr, core::mem::size_of::<T>());
        Self {
            ptr: addr as *const T,
        }
    }
    pub fn inc_and_check(&mut self, offset: usize) {
        let len = core::mem::size_of::<T>();
        self.ptr = (self.ptr as usize + offset * len) as *const T;
        validate_slice(self.ptr as usize, len)
    }
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }
    pub fn as_ptr_mut(&self) -> *mut T {
        self.ptr as *mut T
    }
    pub fn as_ref(&self) -> &T {
        unsafe { &*self.ptr }
    }
    pub fn as_ref_mut(&self) -> &mut T {
        unsafe { &mut *(self.ptr as *mut T) }
    }
    pub fn as_slice_mut(&self, len: usize) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr as *mut T, len) }
    }
}
