//! Page Table Entry

use core::fmt::{self, Debug, Formatter};

use bitflags::bitflags;

use super::address::PhysPageNum;
use crate::{config::mm::PPN_WIDTH, pte_flags};

type FlagInnerType = u16;
const PTE_WIDTH: usize = 10;

bitflags! {
    /// page table entry flags
    #[derive(Clone, Copy, Debug)]
    pub struct PTEFlags: FlagInnerType {
        /// valid
        const V = 1 << 0;
        /// readable
        const R = 1 << 1;
        /// writable
        const W = 1 << 2;
        /// executable
        const X = 1 << 3;
        /// user accessible
        const U = 1 << 4;
        /// global
        const G = 1 << 5;
        /// accessed
        const A = 1 << 6;
        /// dirty
        const D = 1 << 7;
        /// copy-on-write
        const COW = 1 << 8;
        /// lazy-mmap file (unallocated areas)
        const MMAP_FILE = 1 << 9;
    }
}

impl PTEFlags {
    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.contains(Self::V)
    }
    #[inline(always)]
    pub fn is_readable(&self) -> bool {
        self.contains(Self::R)
    }
    #[inline(always)]
    pub fn is_writable(&self) -> bool {
        self.contains(Self::W)
    }
    #[inline(always)]
    pub fn is_executable(&self) -> bool {
        self.contains(Self::X)
    }
    #[inline(always)]
    pub fn is_user(&self) -> bool {
        self.contains(Self::U)
    }
    #[inline(always)]
    pub fn is_global(&self) -> bool {
        self.contains(Self::G)
    }
    #[inline(always)]
    pub fn is_accessed(&self) -> bool {
        self.contains(Self::A)
    }
    #[inline(always)]
    pub fn is_dirty(&self) -> bool {
        self.contains(Self::D)
    }
    #[inline(always)]
    pub fn is_cow(&self) -> bool {
        self.contains(Self::COW)
    }
    pub fn is_mmap_file(&self) -> bool {
        self.contains(Self::MMAP_FILE)
    }
    #[inline(always)]
    pub fn switch_to_cow(&self) -> Self {
        *self & !pte_flags!(W) | pte_flags!(COW)
    }
    #[inline(always)]
    pub fn switch_to_rw(&self) -> Self {
        *self & !pte_flags!(COW) | pte_flags!(W)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(pub usize);

impl PageTableEntry {
    /// create a new page table entry from ppn and flags
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self(ppn.0 << PTE_WIDTH | flags.bits() as usize)
    }
    /// get the physical page number
    pub fn ppn(&self) -> PhysPageNum {
        (self.0 >> PTE_WIDTH & ((1usize << PPN_WIDTH) - 1)).into()
    }
    /// get the pte permission flags
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits((self.0 & ((1 << PTE_WIDTH) - 1)) as FlagInnerType).unwrap()
    }
    /// set flags
    pub fn set_flags(&mut self, flags: PTEFlags) {
        self.0 = (self.0 & !((1 << PTE_WIDTH) - 1)) | (flags.bits() as usize);
    }
    /// clear all data
    pub fn reset(&mut self) {
        self.0 = 0;
    }
}

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "ppn: {:#x} flags: {:?}",
            self.ppn().0,
            self.flags()
        ))
    }
}
