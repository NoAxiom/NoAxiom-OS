//! Physical and virtual address types.

use core::fmt::Debug;

use arch::{
    consts::{INDEX_LEVELS, KERNEL_ADDR_OFFSET, VA_WIDTH},
    PageTableEntry,
};
use config::mm::*;
use include::{
    errno::{Errno, SysResult},
    return_errno,
};

use crate::utils::{kernel_va_to_pa, kernel_vpn_to_ppn};

/// addr type def
/// note that the highter bits of pagenum isn't used and it can be any value
/// but for address, it should be same as the highest bit of the valid address
macro_rules! gen_new_type {
    ($name:ident) => {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
        pub struct $name(pub(crate) usize);
    };
}
gen_new_type!(PhysAddr);
gen_new_type!(VirtAddr);
gen_new_type!(PhysPageNum);
gen_new_type!(VirtPageNum);

impl PhysAddr {
    #[inline(always)]
    pub const fn raw(&self) -> usize {
        self.0
    }
}
impl VirtAddr {
    #[inline(always)]
    pub const fn raw(&self) -> usize {
        self.0
    }
}
impl PhysPageNum {
    #[inline(always)]
    pub const fn raw(&self) -> usize {
        self.0
    }
}
impl VirtPageNum {
    #[inline(always)]
    pub const fn raw(&self) -> usize {
        self.0
    }
}

/// signed extend for number without 64/32 bits width
#[inline(always)]
pub fn signed_extend(num: usize, width: usize) -> usize {
    if num & (1 << (width - 1)) != 0 {
        num | (!((1 << width) - 1))
    } else {
        num
    }
}

/// addr -> usize
macro_rules! impl_from_types {
    ($from:ty, | $param:ident | $body:expr) => {
        impl From<$from> for usize {
            fn from($param: $from) -> Self {
                $body
            }
        }
    };
}
impl_from_types!(PhysAddr, |x| x.0);
impl_from_types!(PhysPageNum, |x| x.0);
impl_from_types!(VirtPageNum, |x| x.0);
impl_from_types!(VirtAddr, |x| signed_extend(x.0, VA_WIDTH));

/// usize -> addr
macro_rules! impl_from_usize {
    ($to:ty) => {
        impl From<usize> for $to {
            fn from(x: usize) -> Self {
                Self(x)
            }
        }
    };
}
impl_from_usize!(PhysAddr);
impl_from_usize!(VirtAddr);
impl_from_usize!(PhysPageNum);
impl_from_usize!(VirtPageNum);

/// add & sub
macro_rules! impl_add_sub {
    ($name:ty, $offset:ty, | $param:ident | $body:expr) => {
        impl core::ops::Add<$offset> for $name {
            type Output = Self;
            fn add(self, $param: $offset) -> Self {
                Self(self.0 + $body)
            }
        }
        impl core::ops::Sub<$offset> for $name {
            type Output = Self;
            fn sub(self, $param: $offset) -> Self {
                Self(self.0 - $body)
            }
        }
    };
}
impl_add_sub!(VirtAddr, usize, |offset| offset);
impl_add_sub!(VirtAddr, VirtAddr, |offset| offset.0);

/// virtual address
impl VirtAddr {
    pub fn offset(&self) -> usize {
        self.0 & ((1 << PAGE_WIDTH) - 1)
    }
    pub fn is_aligned(&self) -> bool {
        self.offset() == 0
    }
    pub fn kernel_translate_into_pa(&self) -> PhysAddr {
        let pa = kernel_va_to_pa(self.0);
        PhysAddr::from(pa)
    }
    pub fn is_in_mmap_range(&self) -> bool {
        MMAP_BASE_ADDR <= self.0 && self.0 <= MMAP_MAX_END_ADDR
    }
}

/// virtual page number
impl VirtPageNum {
    pub fn get_index(&self) -> [usize; INDEX_LEVELS] {
        let mut vpn = self.0;
        let mut idx = [0usize; INDEX_LEVELS];
        for i in (0..INDEX_LEVELS).rev() {
            const MASK: usize = (1 << PAGE_NUM_WIDTH) - 1;
            idx[i] = vpn & MASK;
            vpn >>= PAGE_NUM_WIDTH;
        }
        idx
    }
    #[inline(always)]
    pub fn kernel_translate_into_ppn(&self) -> PhysPageNum {
        let pa = kernel_vpn_to_ppn(self.0);
        PhysPageNum::from(pa)
    }
    #[inline(always)]
    pub fn as_va_usize(&self) -> usize {
        self.0 << PAGE_WIDTH
    }
}

/// physical address
impl PhysAddr {
    pub fn offset(&self) -> usize {
        self.0 & ((1 << PAGE_WIDTH) - 1)
    }
    pub fn is_aligned(&self) -> bool {
        self.offset() == 0
    }
    /// SAFETY: only for kernel space
    pub unsafe fn as_ref<T>(&self) -> &'static T {
        unsafe {
            ((self.0 | KERNEL_ADDR_OFFSET) as *const T)
                .as_ref()
                .unwrap()
        }
    }
    /// SAFETY: only for kernel space
    pub unsafe fn as_mut<T>(&self) -> &'static mut T {
        unsafe { ((self.0 | KERNEL_ADDR_OFFSET) as *mut T).as_mut().unwrap() }
    }
}

/// physical page number
/// SAFETY: be very careful if you want to convert pa into an ptr
/// since the pa should be added KERNEL_ADDR_OFFSET to get the va
/// all ptr conversion should only be used to fetch kernel address
/// and in any conversion, the func should added KERNEL_ADDR_OFFSET to pa
impl PhysPageNum {
    /// convert self into physical address
    pub fn into_pa(&self) -> PhysAddr {
        (*self).into()
    }
    /// get pte array from self pointing address
    /// SAFETY: only for kernel space
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        trace!("get_pte_array: ppn = {:#x}", self.0);
        unsafe {
            core::slice::from_raw_parts_mut(
                (self.into_pa().0 | KERNEL_ADDR_OFFSET) as *mut PageTableEntry,
                PTE_PER_PAGE,
            )
        }
    }
    /// get bytes array from self pointing address
    /// SAFETY: only for kernel space
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                (self.into_pa().0 | KERNEL_ADDR_OFFSET) as *mut u8,
                PAGE_SIZE,
            )
        }
    }
}

/// addr <> page_num
macro_rules! impl_mutual_convert {
    ($from:ident, $to:ident) => {
        impl $from {
            pub fn floor(&self) -> $to {
                $to(self.0 >> PAGE_WIDTH)
            }
            pub fn ceil(&self) -> $to {
                $to((self.0 + PAGE_SIZE - 1) >> PAGE_WIDTH)
            }
        }
        impl From<$from> for $to {
            fn from(x: $from) -> Self {
                assert!(x.is_aligned(), "addr {:#x} is not an aligned page!", x.0);
                x.floor()
            }
        }
        impl From<$to> for $from {
            fn from(x: $to) -> Self {
                Self(x.0 << PAGE_WIDTH)
            }
        }
    };
}
impl_mutual_convert!(VirtAddr, VirtPageNum);
impl_mutual_convert!(PhysAddr, PhysPageNum);

/// virtual page number range,
/// which is used to iterate over vpn ranges
#[derive(Clone, Copy)]
pub struct VpnRange {
    start: VirtPageNum,
    end: VirtPageNum,
}
impl VpnRange {
    pub fn new(start: VirtPageNum, end: VirtPageNum) -> SysResult<Self> {
        if start > end {
            return_errno!(Errno::EFAULT, "start {:#x?} > end {:#x?}!", start, end);
        }
        Ok(Self { start, end })
    }
    #[inline(always)]
    pub fn new_from_va(start_va: VirtAddr, end_va: VirtAddr) -> SysResult<Self> {
        Self::new(start_va.floor(), end_va.ceil())
    }
    #[inline(always)]
    pub const fn start(&self) -> VirtPageNum {
        self.start
    }
    #[inline(always)]
    pub const fn end(&self) -> VirtPageNum {
        self.end
    }
    #[inline(always)]
    pub fn is_in_range(&self, vpn: VirtPageNum) -> bool {
        self.start <= vpn && vpn < self.end
    }
    #[inline(always)]
    pub fn page_count(&self) -> usize {
        self.end.0 - self.start.0
    }
}

impl Debug for VpnRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "[{:#x}, {:#x})",
            VirtAddr::from(self.start).0,
            VirtAddr::from(self.end).0
        )
    }
}

/// step one and return the previous value,
/// for iterator usage
pub trait StepOne {
    fn step(&mut self) -> Self;
}
impl StepOne for VirtPageNum {
    fn step(&mut self) -> Self {
        let tmp = self.clone();
        self.0 += 1;
        tmp
    }
}
impl StepOne for PhysPageNum {
    fn step(&mut self) -> Self {
        let tmp = self.clone();
        self.0 += 1;
        tmp
    }
}

/// iterator for vpn range
impl IntoIterator for VpnRange {
    type Item = VirtPageNum;
    type IntoIter = IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            next: self.start(),
            end: self.end(),
        }
    }
}
pub struct IntoIter<T> {
    next: T,
    end: T,
}
impl<T> Iterator for IntoIter<T>
where
    T: PartialEq + StepOne,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.end {
            None
        } else {
            Some(self.next.step())
        }
    }
}
