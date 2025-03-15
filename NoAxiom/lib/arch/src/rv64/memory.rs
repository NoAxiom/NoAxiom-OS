use core::arch::asm;

use bitflags::bitflags;
use riscv::{asm::sfence_vma_all, register::satp};

use super::RV64;
use crate::{utils::macros::bit, ArchMemory, ArchPTE, ArchPageTable, MappingFlags};

mod sv39 {
    /// physical address width
    pub const PA_WIDTH: usize = 56;
    /// virtual address width
    pub const VA_WIDTH: usize = 39;
    /// index level number of sv39
    pub const INDEX_LEVELS: usize = 3;
}

pub struct PageTable {
    root_ppn: usize,
}

#[derive(Clone, Copy)]
pub struct PageTableEntry {
    inner: usize,
}
impl Into<usize> for PageTableEntry {
    fn into(self) -> usize {
        self.inner
    }
}
impl From<usize> for PageTableEntry {
    fn from(value: usize) -> Self {
        Self { inner: value }
    }
}
impl ArchPTE for PageTableEntry {}

impl ArchPageTable for PageTable {
    type PTEFlags = PTEFlags;
    type PageTableEntry = PageTableEntry;
    const PA_WIDTH: usize = sv39::PA_WIDTH;
    const VA_WIDTH: usize = sv39::VA_WIDTH;
    const INDEX_LEVELS: usize = sv39::INDEX_LEVELS;
    fn new(root_ppn: usize) -> Self {
        Self { root_ppn }
    }
    fn root_ppn(&self) -> usize {
        self.root_ppn
    }
    fn activate(&self) {
        satp::write(8usize << 60 | self.root_ppn);
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PTEFlags: u64 {
        const V = bit!(0);
        const R = bit!(1);
        const W = bit!(2);
        const X = bit!(3);
        const U = bit!(4);
        const G = bit!(5);
        const A = bit!(6);
        const D = bit!(7);
        const COW = bit!(8);

        const VRWX  = Self::V.bits() | Self::R.bits() | Self::W.bits() | Self::X.bits();
        const ADUVRX = Self::A.bits() | Self::D.bits() | Self::U.bits() | Self::V.bits() | Self::R.bits() | Self::X.bits();
        const ADVRWX = Self::A.bits() | Self::D.bits() | Self::VRWX.bits();
        const ADGVRWX = Self::G.bits() | Self::ADVRWX.bits();
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(flags: MappingFlags) -> Self {
        if flags.is_empty() {
            Self::empty()
        } else {
            let mut res = Self::V;
            if flags.contains(MappingFlags::R) {
                res |= PTEFlags::R | PTEFlags::A;
            }
            if flags.contains(MappingFlags::W) {
                res |= PTEFlags::W | PTEFlags::D;
            }
            if flags.contains(MappingFlags::X) {
                res |= PTEFlags::X;
            }
            if flags.contains(MappingFlags::U) {
                res |= PTEFlags::U;
            }
            res
        }
    }
}

impl From<PTEFlags> for MappingFlags {
    fn from(value: PTEFlags) -> Self {
        let mut mapping_flags = MappingFlags::empty();
        if value.contains(PTEFlags::V) {
            mapping_flags |= MappingFlags::P;
        }
        if value.contains(PTEFlags::R) {
            mapping_flags |= MappingFlags::R;
        }
        if value.contains(PTEFlags::W) {
            mapping_flags |= MappingFlags::W;
        }
        if value.contains(PTEFlags::X) {
            mapping_flags |= MappingFlags::X;
        }
        if value.contains(PTEFlags::U) {
            mapping_flags |= MappingFlags::U;
        }
        // fixme: G???
        if value.contains(PTEFlags::A) {
            mapping_flags |= MappingFlags::A;
        }
        if value.contains(PTEFlags::D) {
            mapping_flags |= MappingFlags::D;
        }
        if value.contains(PTEFlags::COW) {
            mapping_flags |= MappingFlags::COW;
        }
        mapping_flags
    }
}

impl ArchMemory for RV64 {
    type PageTable = PageTable;
    // flush all TLB
    #[inline(always)]
    fn tlb_flush() {
        sfence_vma_all();
    }
    #[inline(always)]
    fn current_root_ppn() -> usize {
        let satp: usize;
        unsafe { asm!("csrr {}, satp", out(reg) satp) }
        satp & ((1 << PageTable::PPN_WIDTH) - 1)
    }
    #[inline(always)]
    fn activate(ppn: usize) {
        satp::write(8usize << 60 | ppn)
    }
}
