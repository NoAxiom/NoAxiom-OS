use core::slice::from_raw_parts_mut;

use riscv::register::scause::Exception;

use super::{address::VirtAddr, memory_set::MemorySet};
use crate::{
    config::mm::PAGE_SIZE,
    cpu::current_cpu,
    mm::address::{VirtPageNum, VpnRange},
    nix::result::Errno,
    syscall::SyscallResult,
    utils::align_up,
};

// TODO: add mmap check
/// # memory validate
/// Check if is the copy-on-write/lazy-alloc pages triggered the page fault.
///
/// As for cow, clone pages for the writer(aka current task),
/// but should keep original page as cow since it might still be shared.
/// Note that if the reference count is one, there's no need to clone pages.
///
/// As for lazy alloc, realloc pages for the task.
/// Associated pages: stack, heap, mmap
///
/// Return value: true if successfully handled lazy alloc or copy-on-write;
///               false if the page fault is not in any alloc area.
///
/// usages: when any kernel allocation in user_space happens, call this fn;
/// when user pagefault happens, call this func to check allocation.
pub fn validate(
    addr: usize,
    memory_set: &mut MemorySet,
    exception: Option<Exception>,
) -> SyscallResult {
    let vpn = VirtAddr::from(addr).floor();
    if let Some(pte) = memory_set.page_table().translate_vpn(vpn) {
        let flags = pte.flags();
        if flags.is_cow() {
            info!(
                "[memory_validate] realloc COW at va: {:#x}, pte: {:#x}, flags: {:?}",
                addr,
                pte.0,
                pte.flags()
            );
            memory_set.realloc_cow(vpn, pte);
            Ok(0)
        } else if exception.is_some() && exception.unwrap() == Exception::StorePageFault {
            error!(
                "page fault at addr: {:#x}, store at invalid area, flags: {:?}",
                addr, flags
            );
            Err(Errno::EFAULT)
        } else {
            Ok(0)
        }
    } else {
        if memory_set.user_stack_area.vpn_range.is_in_range(vpn) {
            info!("[memory_validate] realloc stack");
            memory_set.realloc_stack(vpn);
            Ok(0)
        } else if memory_set.user_heap_area.vpn_range.is_in_range(vpn) {
            info!("[memory_validate] realloc heap");
            memory_set.realloc_heap(vpn);
            Ok(0)
        } else {
            error!("page fault at addr: {:#x}, not in any alloc area", addr);
            Err(Errno::EFAULT)
        }
    }
}

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
        let _ = task.memory_validate(vpn.as_va_usize(), None);
    }
}

/// the UserPtr is a wrapper for user-space pointer
/// it will validate the pointer before we access it
pub struct UserPtr<T> {
    ptr: *const T,
}

impl<T> UserPtr<T> {
    pub fn new(addr: usize) -> Self {
        validate_slice(addr, core::mem::size_of::<T>());
        Self {
            ptr: addr as *const T,
        }
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
    /// SAFETY: this is only used for user-write pages
    /// any unallcated memory access should be handled before this
    pub unsafe fn as_slice_mut_unchecked(&self, len: usize) -> &mut [T] {
        from_raw_parts_mut(self.ptr as *mut T, len)
    }
    pub fn as_slice_mut(&self, len: usize) -> &mut [T] {
        validate_slice(self.ptr as usize, len);
        unsafe { from_raw_parts_mut(self.ptr as *mut T, len) }
    }
    /// SAFETY: this is only used for user-write pages
    /// any unallcated memory access should be handled before this
    pub unsafe fn as_slice_while_unchecked(&self, checker: &dyn Fn(&T) -> bool) -> &mut [T] {
        let start = self.ptr as usize;
        let mut ptr = self.ptr as usize;
        loop {
            if !checker(&*(ptr as *const T)) {
                break;
            }
            ptr += 1;
        }
        from_raw_parts_mut(self.ptr as *mut T, ptr - start)
    }
    pub fn as_slice_while(&self, checker: &dyn Fn(&T) -> bool) -> &mut [T] {
        let start = self.ptr as usize;
        let mut ptr = self.ptr as usize;
        let mut memory_set = current_cpu().task.as_mut().unwrap().memory_set().lock();
        let mut page_end = align_up(ptr + 1, PAGE_SIZE);
        let _ = validate(ptr as usize, &mut memory_set, None);
        debug!("[as_slice_mut] {:#x} end: {:#x}", ptr, page_end);
        loop {
            let ptr_end = ptr + core::mem::size_of::<T>() - 1;
            if ptr_end >= page_end {
                warn!(
                    "[as_slice_while] page_end: {:#x}, ptr_end: {:#x}",
                    page_end, ptr_end
                );
                let _ = validate(ptr_end as usize, &mut memory_set, None);
                page_end += PAGE_SIZE;
            }
            if !checker(unsafe { &*(ptr as *const T) }) {
                break;
            }
            ptr += 1;
        }
        unsafe { from_raw_parts_mut(self.ptr as *mut T, ptr - start) }
    }
}
