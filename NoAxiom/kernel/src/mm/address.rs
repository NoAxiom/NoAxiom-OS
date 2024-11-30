//! Physical and virtual address types.

use super::pte::PageTableEntry;
use crate::{
    config::mm::*,
    utils::{kernel_va_to_pa, kernel_vpn_to_ppn, signed_extend},
};

/// addr type def
macro_rules! gen_new_type {
    ($name:ident) => {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
        pub struct $name(pub usize);
    };
}
gen_new_type!(PhysAddr);
gen_new_type!(VirtAddr);
gen_new_type!(PhysPageNum);
gen_new_type!(VirtPageNum);

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
}

/// virtual page number
impl VirtPageNum {
    pub fn get_index(&self) -> [usize; INDEX_LEVELS] {
        let mut vpn = self.0;
        let mut idx = [0; INDEX_LEVELS];
        for i in (0..INDEX_LEVELS).rev() {
            idx[i] = vpn & ((1 << PAGE_NUM_WIDTH) - 1);
            vpn >>= PAGE_NUM_WIDTH;
        }
        idx
    }
    pub fn kernel_translate_into_ppn(&self) -> PhysPageNum {
        let pa = kernel_vpn_to_ppn(self.0);
        PhysPageNum::from(pa)
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
    pub fn as_ref<T>(&self) -> &'static T {
        unsafe { (self.0 as *const T).as_ref().unwrap() }
    }
    pub fn as_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }
}

/// physical page number
impl PhysPageNum {
    /// convert self into physical address
    pub fn into_pa(&self) -> PhysAddr {
        (*self).into()
    }
    /// get pte array from self pointing address
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        unsafe {
            core::slice::from_raw_parts_mut(self.into_pa().0 as *mut PageTableEntry, PTE_PER_PAGE)
        }
    }
    /// get bytes array from self pointing address
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.into_pa().0 as *mut u8, PAGE_SIZE) }
    }
    /// return self as a mut generic reference
    pub fn as_mut<T>(&self) -> &'static mut T {
        self.into_pa().as_mut()
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
                assert!(x.is_aligned(), "addr {:?} is not an aligned page!", x);
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
#[derive(Clone, Copy, Debug)]
pub struct VpnRange {
    start: VirtPageNum,
    end: VirtPageNum,
}
impl VpnRange {
    pub fn new(start: VirtPageNum, end: VirtPageNum) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { start, end }
    }
    pub fn new_from_va(start_va: VirtAddr, end_va: VirtAddr) -> Self {
        let start = start_va.floor();
        let end = end_va.ceil();
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { start, end }
    }
    pub fn start(&self) -> VirtPageNum {
        self.start
    }
    pub fn end(&self) -> VirtPageNum {
        self.end
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

/// iterator for vpn range
impl IntoIterator for VpnRange {
    type Item = VirtPageNum;
    type IntoIter = IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            next: self.start,
            end: self.end,
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
