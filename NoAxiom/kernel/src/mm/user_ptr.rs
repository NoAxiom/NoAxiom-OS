use alloc::{string::String, vec::Vec};
use core::{intrinsics::atomic_load_acquire, marker::PhantomData};

use arch::{
    Arch, ArchMemory, ArchPageTableEntry, ArchTrap, ExceptionType, MappingFlags, PageFaultType,
    TrapType,
};
use config::mm::PAGE_SIZE;
use include::errno::Errno;
use ksync::mutex::check_no_lock;
use memory::address::PhysAddr;

use super::{address::VirtAddr, page_table::PageTable, validate::validate};
use crate::{
    cpu::current_task,
    mm::address::VpnRange,
    return_errno,
    sched::utils::block_on,
    syscall::{utils::current_syscall, SysResult},
};

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
    #[inline(always)]
    pub const fn new(addr: usize) -> Self {
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
    pub fn is_non_null(&self) -> bool {
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

    pub const fn ptr_will_cross_page(addr: usize) -> bool {
        let next_addr = addr + core::mem::size_of::<T>();
        let current_page = addr & !(PAGE_SIZE - 1);
        let next_page = next_addr & !(PAGE_SIZE - 1);
        current_page != next_page
    }

    pub const fn will_cross_page(&self) -> bool {
        Self::ptr_will_cross_page(self.addr())
    }

    pub async fn get_ref_mut(&self) -> SysResult<Option<&mut T>> {
        match unsafe { self.ptr().as_mut() } {
            Some(ptr) => {
                self.check_write().await?;
                Ok(Some(ptr))
            }
            None => Ok(None),
        }
    }

    pub unsafe fn read_unchecked(&self) -> T
    where
        T: Copy,
    {
        unsafe { *self.ptr() }
    }

    pub async fn check_read(&self) -> SysResult<()> {
        if self.is_null() {
            warn!("[read] read null pointer");
            return Err(Errno::EFAULT);
        }
        match Arch::check_read(self.addr()) {
            Ok(()) => Ok(()),
            Err(trap_type) => {
                match trap_type {
                    TrapType::Exception(ExceptionType::PageFault(pf)) => match pf {
                        PageFaultType::LoadPageFault(addr)
                        | PageFaultType::StorePageFault(addr) => {
                            warn!(
                                "[read] detect trap at addr {:#x} during syscall {:?}",
                                self.addr(),
                                current_syscall()
                            );
                            let task = current_task().unwrap();
                            if check_no_lock() {
                                task.memory_validate(addr, pf, false).await?;
                            } else {
                                warn!(
                                    "[read] block on addr {:#x} during syscall {:?}",
                                    self.addr(),
                                    current_syscall()
                                );
                                block_on(task.memory_validate(addr, pf, true))?;
                            }
                            return Ok(());
                        }
                        _ => {}
                    },
                    _ => {}
                }
                return_errno!(
                    Errno::EFAULT,
                    "[user_ptr] trigger unexpected trap in read, trap_type: {:?}",
                    trap_type
                );
            }
        }
    }

    pub async fn read(&self) -> SysResult<T>
    where
        T: Copy,
    {
        self.check_read().await?;
        Ok(unsafe { self.read_unchecked() })
    }

    pub async fn try_read(&self) -> SysResult<Option<T>>
    where
        T: Copy,
    {
        match self.ptr().is_null() {
            false => {
                let res = self.read().await?;
                Ok(Some(res))
            }
            true => Ok(None),
        }
    }

    pub unsafe fn atomic_load_acquire(&self) -> T
    where
        T: Copy,
    {
        unsafe { atomic_load_acquire(self.ptr()) }
    }

    pub unsafe fn write_unchecked(&self, value: T) {
        unsafe { *self.ptr() = value }
    }

    pub async fn check_write(&self) -> SysResult<()> {
        if self.is_null() {
            warn!("[write] write null pointer");
            return Err(Errno::EFAULT);
        }
        match Arch::check_write(self.addr()) {
            Ok(()) => Ok(()),
            Err(trap_type) => {
                match trap_type {
                    TrapType::Exception(ExceptionType::PageFault(pf)) => match pf {
                        PageFaultType::LoadPageFault(addr)
                        | PageFaultType::StorePageFault(addr) => {
                            warn!(
                                "[write] detect trap at addr {:#x} during syscall {:?}",
                                self.addr(),
                                current_syscall()
                            );
                            let task = current_task().unwrap();
                            if check_no_lock() {
                                task.memory_validate(addr, pf, false).await?;
                            } else {
                                warn!(
                                    "[write] block on addr {:#x} during syscall {:?}",
                                    self.addr(),
                                    current_syscall()
                                );
                                block_on(task.memory_validate(addr, pf, true))?;
                            }
                            return Ok(());
                        }
                        _ => {}
                    },
                    _ => {}
                }
                return_errno!(
                    Errno::EFAULT,
                    "[user_ptr] trigger unexpected trap in write, trap_type: {:?}",
                    trap_type
                );
            }
        }
    }

    pub async fn write(&self, value: T) -> SysResult<()> {
        self.check_write().await?;
        Ok(unsafe { self.write_unchecked(value) })
    }

    pub async fn try_write(&self, value: T) -> SysResult<Option<()>> {
        match self.ptr().is_null() {
            false => {
                self.write(value).await?;
                Ok(Some(()))
            }
            true => Ok(None),
        }
    }

    /// get user slice until the checker returns true
    pub fn clone_as_vec_until(&self, checker: impl Fn(&T) -> bool) -> SysResult<Vec<T>>
    where
        T: Copy,
    {
        let mut ptr = self.addr as usize;
        let mut res = Vec::new();
        let step = core::mem::size_of::<T>();
        let mut cross_flag = true;
        loop {
            let u_ptr = Self::from(ptr);
            if cross_flag {
                block_on(u_ptr.read())?;
            }
            trace!("[as_vec_while] ptr: {:#x}", ptr);
            let value = unsafe { &*(ptr as *const T) };
            if checker(value) {
                break;
            }
            res.push(*value);
            cross_flag = u_ptr.will_cross_page();
            ptr += step;
        }
        Ok(res)
    }

    pub async fn as_slice_const_checked<'a>(&self, len: usize) -> SysResult<&[T]> {
        let ptr_u8 = UserPtr::<u8>::new(self.addr as usize);
        let len_u8 = len * core::mem::size_of::<T>();
        let slice = ptr_u8.as_slice_checked_raw(len_u8, false).await?;
        Ok(unsafe { core::slice::from_raw_parts(slice.as_ptr() as *const T, len) })
    }

    pub async fn as_slice_mut_checked<'a>(&self, len: usize) -> SysResult<&mut [T]> {
        let ptr_u8 = UserPtr::<u8>::new(self.addr as usize);
        let len_u8 = len * core::mem::size_of::<T>();
        let slice = ptr_u8.as_slice_checked_raw(len_u8, true).await?;
        Ok(unsafe { core::slice::from_raw_parts_mut(slice.as_ptr() as *mut T, len) })
    }

    async fn raw_validate(&self, is_write: bool) -> SysResult<()> {
        let _ = match is_write {
            false => self.as_slice_const_checked(1).await?,
            true => self.as_slice_mut_checked(1).await?,
        };
        Ok(())
    }

    /// validate the user pointer
    /// this will check the page table and allocate valid map areas
    /// or it will return EFAULT
    #[deprecated(note = "use check_read / check_write instead")]
    #[allow(dead_code)]
    pub async fn validate(&self, is_write: bool) -> SysResult<()> {
        if check_no_lock() {
            self.raw_validate(is_write).await?;
        } else {
            warn!(
                "[validate] block on addr {:#x} during syscall {:?}",
                self.addr(),
                current_syscall()
            );
            block_on(self.raw_validate(is_write))?;
        }
        Ok(())
    }

    /// translate current address to physical address
    pub async fn translate_pa(&self) -> SysResult<PhysAddr> {
        if let Err(TrapType::Exception(ExceptionType::PageFault(pf))) =
            Arch::check_read(self.addr())
        {
            warn!(
                "[translate_pa] detect trap at addr {:#x} during syscall {:?}",
                self.addr(),
                current_syscall()
            );
            let task = current_task().unwrap();
            if check_no_lock() {
                task.memory_validate(self.addr(), pf, false).await?;
            } else {
                warn!(
                    "[translate_pa] block on addr {:#x} during syscall {:?}",
                    self.addr(),
                    current_syscall()
                );
                block_on(task.memory_validate(self.addr(), pf, true))?;
            }
        }
        let page_table = PageTable::from_ppn(Arch::current_root_ppn());
        page_table.translate_va(self.va_addr()).ok_or(Errno::EFAULT)
    }
}

impl UserPtr<u8> {
    /// get user string
    pub fn get_cstr(&self) -> SysResult<String> {
        let slice = self.clone_as_vec_until(|&c: &u8| c as char == '\0')?;
        trace!("slice: {:?}", slice);
        let res = unsafe { String::from_utf8_unchecked(Vec::from(slice)) };
        Ok(res)
    }

    /// convert ptr into an slice
    async fn as_slice_checked_raw<'a>(&self, len: usize, is_write: bool) -> SysResult<&mut [u8]> {
        let page_table = PageTable::from_ppn(Arch::current_root_ppn());
        let memory_set = current_task().unwrap().memory_set();
        let range = VpnRange::new_from_va(
            VirtAddr::from(self.addr()),
            VirtAddr::from(self.addr() + len),
        )?;
        for vpn in range {
            let pte = page_table.find_pte(vpn);
            let should_validate = if pte.is_none() {
                true
            } else {
                is_write && !pte.unwrap().flags().contains(MappingFlags::W)
            };
            if should_validate {
                let trap_type = match is_write {
                    true => PageFaultType::StorePageFault(vpn.as_va_usize()),
                    false => PageFaultType::LoadPageFault(vpn.as_va_usize()),
                };
                validate(memory_set, vpn, trap_type, None).await?;
            }
        }
        Ok(unsafe { core::slice::from_raw_parts_mut(self.ptr(), len) })
    }

    /// get string from u8 ptr
    /// this function only requires read permission
    pub fn get_string_from_ptr(&self) -> SysResult<String> {
        let checker = |&c: &u8| c == 0;
        let slice = self.clone_as_vec_until(checker)?;
        let res = unsafe { String::from_utf8_unchecked(Vec::from(slice)) };
        Ok(res)
    }
}

impl UserPtr<UserPtr<u8>> {
    /// get user string vec, end with null
    pub async fn get_string_vec(&self) -> SysResult<Vec<String>> {
        let mut ptr = self.clone();
        let mut res = Vec::new();
        while !ptr.is_null() && !ptr.read().await?.is_null() {
            trace!(
                "ptr_addr: {:#}, value: {:#}",
                ptr.va_addr().raw(),
                ptr.read().await?.va_addr().raw()
            );
            let data = ptr.read().await?.get_cstr()?;
            res.push(data);
            ptr.inc(1);
        }
        Ok(res)
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
