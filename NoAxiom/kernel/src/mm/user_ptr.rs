use alloc::vec::Vec;

use riscv::register::scause::Exception;

use super::{address::VirtAddr, memory_set::MemorySet};
use crate::{
    config::mm::KERNEL_ADDR_OFFSET,
    cpu::current_cpu,
    mm::address::{VirtPageNum, VpnRange},
    nix::result::Errno,
    syscall::SyscallResult,
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
        let _ = validate(vpn.as_va_usize(), &mut memory_set, None);
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

    /// convert ptr into an mutable reference
    /// please write data after memory_set.unlock
    pub unsafe fn as_ref_mut(&self) -> &mut T {
        unsafe { &mut *(self.ptr as *mut T) }
    }

    /// clone a slice as vec from user space
    pub unsafe fn as_vec(&self, len: usize) -> Vec<T>
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
    pub unsafe fn as_vec_until(&self, checker: &dyn Fn(&T) -> bool) -> Vec<T>
    where
        T: Copy,
    {
        let mut ptr = self.ptr as usize;
        let mut res = Vec::new();
        let step = core::mem::size_of::<T>();
        debug!("[as_vec_while] ptr: {:#x}", ptr);
        loop {
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
