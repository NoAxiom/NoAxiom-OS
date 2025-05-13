use alloc::{string::String, vec::Vec};
use core::{intrinsics::atomic_load_acquire, marker::PhantomData};

use arch::{consts::KERNEL_ADDR_OFFSET, Arch, ArchMemory};
use include::errno::Errno;
use memory::address::PhysAddr;

use super::{address::VirtAddr, page_table::PageTable, validate::validate};
use crate::{cpu::current_task, mm::address::VpnRange, syscall::SysResult};

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
pub struct UserPtr<T = u8> {
    _phantom: PhantomData<T>,
    addr: usize,
}

impl<T> UserPtr<T> {
    pub fn new(addr: usize) -> Self {
        assert!(
            addr & KERNEL_ADDR_OFFSET == 0,
            "shouldn't pass kernel address"
        );
        Self {
            _phantom: PhantomData,
            addr,
        }
    }

    #[inline(always)]
    pub const fn ptr(&self) -> *mut T {
        self.addr as *mut T
    }

    #[inline(always)]
    pub fn new_null() -> Self {
        Self::new(0)
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.ptr().is_null()
    }

    #[inline(always)]
    pub fn not_null(&self) -> bool {
        !self.ptr().is_null()
    }

    #[inline(always)]
    pub fn inc(&mut self, count: usize) {
        self.addr = unsafe { self.ptr().add(count) } as usize;
    }

    #[inline(always)]
    pub fn va_addr(&self) -> VirtAddr {
        VirtAddr::from(self.addr as usize)
    }

    #[inline(always)]
    pub const fn addr(&self) -> usize {
        self.addr as usize
    }

    #[inline(always)]
    pub fn read(&self) -> T
    where
        T: Copy,
    {
        unsafe { *self.ptr() }
    }

    #[inline(always)]
    #[allow(unused)]
    pub fn read_volatile(&self) -> T
    where
        T: Copy,
    {
        unsafe { self.ptr().read_volatile() }
    }

    #[inline(always)]
    pub fn atomic_load_acquire(&self) -> T
    where
        T: Copy,
    {
        unsafe { atomic_load_acquire(self.ptr()) }
    }

    #[inline(always)]
    pub fn write(&self, value: T) {
        unsafe { *self.ptr() = value };
    }

    #[inline(always)]
    #[allow(unused)]
    pub fn write_volatile(&self, value: T) {
        unsafe { self.ptr().write_volatile(value) };
    }

    /// get user slice until the checker returns true
    pub fn clone_as_vec_until(&self, checker: impl Fn(&T) -> bool) -> Vec<T>
    where
        T: Copy,
    {
        let mut ptr = self.addr as usize;
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

    pub async fn as_slice_mut_checked<'a>(&self, len: usize) -> SysResult<&mut [T]> {
        let ptr_u8 = UserPtr::<u8>::new(self.addr as usize);
        let len_u8 = len * core::mem::size_of::<T>();
        let slice = ptr_u8.as_slice_mut_checked_raw(len_u8).await?;
        Ok(unsafe { core::slice::from_raw_parts_mut(slice.as_ptr() as *mut T, len) })
    }

    /// validate the user pointer
    /// this will check the page table and allocate valid map areas
    /// or it will return EFAULT
    pub async fn validate(&self) -> SysResult<()> {
        let _slice = self.as_slice_mut_checked(1).await?;
        Ok(())
    }

    /// translate current address to physical address
    pub async fn translate_pa(&self) -> SysResult<PhysAddr> {
        self.validate().await?;
        let page_table = PageTable::from_ppn(Arch::current_root_ppn());
        page_table.translate_va(self.va_addr()).ok_or(Errno::EFAULT)
    }
}

impl UserPtr<u8> {
    /// get user string
    pub fn get_cstr(&self) -> String {
        let slice = self.clone_as_vec_until(|&c: &u8| c as char == '\0');
        trace!("slice: {:?}", slice);
        let res = String::from_utf8(Vec::from(slice)).unwrap();
        res
    }

    /// convert ptr into an slice
    pub async fn as_slice_mut_checked_raw<'a>(&self, len: usize) -> SysResult<&mut [u8]> {
        let page_table = PageTable::from_ppn(Arch::current_root_ppn());
        let memory_set = current_task().memory_set();
        for vpn in VpnRange::new_from_va(
            VirtAddr::from(self.addr()),
            VirtAddr::from(self.addr() + len),
        ) {
            if page_table.find_pte(vpn).is_none() {
                validate(memory_set, vpn, None, None).await?;
            }
        }
        Ok(unsafe { core::slice::from_raw_parts_mut(self.ptr(), len) })
    }
}

impl UserPtr<UserPtr<u8>> {
    /// get user string vec, end with null
    pub fn get_string_vec(&self) -> Vec<String> {
        let mut ptr = self.clone();
        let mut res = Vec::new();
        while !ptr.is_null() && !ptr.read().is_null() {
            trace!(
                "ptr_addr: {:#}, value: {:#}",
                ptr.va_addr().raw(),
                ptr.read().va_addr().raw()
            );
            let data = ptr.read().get_cstr();
            res.push(data);
            ptr.inc(1);
        }
        res
    }
}

// the userptr is safe to send and sync
unsafe impl<T> Send for UserPtr<T> {}
unsafe impl<T> Sync for UserPtr<T> {}

impl<T> From<usize> for UserPtr<T> {
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

impl<T> From<*const T> for UserPtr<T> {
    fn from(value: *const T) -> Self {
        Self::new(value as usize)
    }
}

impl<T> From<*mut T> for UserPtr<T> {
    fn from(value: *mut T) -> Self {
        Self::new(value as usize)
    }
}
