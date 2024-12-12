//! Physical and virtual address types.

use super::pte::PageTableEntry;
use crate::{
    config::mm::*,
    utils::{kernel_va_to_pa, kernel_vpn_to_ppn, signed_extend},
};

/// addr type def
/// note that the highter bits of pagenum isn't used and it can be any value
/// but for address, it should be same as the highest bit of the valid address
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
    #[inline(always)]
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
    #[inline(always)]
    pub fn new_from_va(start_va: VirtAddr, end_va: VirtAddr) -> Self {
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

// #[allow(unused)]
// pub fn kernel_address_test() {
//     let va = VirtAddr(0x8000_0000);
//     let pa = va.kernel_translate_into_pa();
//     assert_eq!(pa.0, 0x0);

//     let vpn = VirtPageNum(0x80000);
//     let ppn = vpn.kernel_translate_into_ppn();
//     assert_eq!(ppn.0, 0x0);

//     let va = VirtAddr(0x8000_0000);
//     let vpn: VirtPageNum = va.into();
//     assert_eq!(vpn.0, 0x80000);

//     let va = VirtAddr(0x8000_0000);
//     let vpn = VirtPageNum::from(va);
//     assert_eq!(vpn.0, 0x80000);

//     let pa = PhysAddr(0x80000);
//     let ppn: PhysPageNum = pa.into();
//     assert_eq!(ppn.0, 0x80000);

//     let pa = PhysAddr(0x80000);
//     let ppn = PhysPageNum::from(pa);
//     assert_eq!(ppn.0, 0x80000);

//     let pa = PhysAddr(0x80000);
//     let ppn = pa.floor();
//     assert_eq!(ppn.0, 0x80000);

//     let pa = PhysAddr(0x80000);
//     let ppn: PhysPageNum = pa.into();
//     assert_eq!(ppn.0, 0x8);

//     let pa = PhysAddr(0x0);
//     let ppn = PhysPageNum::from(pa);
//     assert_eq!(ppn.0, 0x0);

//     let vpn = VirtPageNum(0x8000_0000);
//     let va = vpn.into_va();
//     assert_eq!(va.0, 0x8000_0000);
// }
