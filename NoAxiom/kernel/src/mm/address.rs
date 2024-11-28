//! Physical and virtual address types.

use crate::{config::mm::*, utils::signed_extend};

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

macro_rules! impl_width {
    ($t:ty, $width:expr) => {
        impl $t {
            pub fn width() -> usize {
                $width
            }
        }
    };
}
impl_width!(PhysAddr, PA_WIDTH);
impl_width!(VirtAddr, VA_WIDTH);
impl_width!(PhysPageNum, PPN_WIDTH);
impl_width!(VirtPageNum, VPN_WIDTH);

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
impl_from_types!(VirtAddr, |x| signed_extend(x.0, VirtAddr::width()));

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

macro_rules! impl_add_sub_usize {
    ($t:ty) => {
        impl core::ops::Add<usize> for $t {
            type Output = Self;
            fn add(self, rhs: usize) -> Self {
                Self(self.0 + rhs)
            }
        }
        impl core::ops::Sub<usize> for $t {
            type Output = Self;
            fn sub(self, rhs: usize) -> Self {
                Self(self.0 - rhs)
            }
        }
    };
}
impl_add_sub_usize!(PhysAddr);
impl_add_sub_usize!(VirtAddr);
impl_add_sub_usize!(PhysPageNum);
impl_add_sub_usize!(VirtPageNum);

macro_rules! impl_add_sub_self {
    ($t:ty) => {
        impl core::ops::Add<$t> for $t {
            type Output = Self;
            fn add(self, rhs: $t) -> Self {
                Self(self.0 + rhs.0)
            }
        }
        impl core::ops::Sub<$t> for $t {
            type Output = Self;
            fn sub(self, rhs: $t) -> Self {
                Self(self.0 - rhs.0)
            }
        }
    };
}
impl_add_sub_self!(PhysAddr);
impl_add_sub_self!(VirtAddr);
impl_add_sub_self!(PhysPageNum);
impl_add_sub_self!(VirtPageNum);

macro_rules! impl_raw_address {
    ($t:ty) => {
        impl $t {
            fn offset(&self) -> usize {
                self.0 & ((1 << PAGE_WIDTH) - 1)
            }
            fn is_aligned(&self) -> bool {
                self.offset() == 0
            }
        }
    };
}
impl_raw_address!(VirtAddr);
impl_raw_address!(PhysAddr);

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
